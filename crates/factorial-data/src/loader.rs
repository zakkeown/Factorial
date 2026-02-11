//! Resolution pipeline: reads data files, resolves cross-references, builds registry.
//!
//! Provides format detection (RON/JSON/TOML), file discovery, and deserialization
//! helpers used by the higher-level loading pipeline.

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ===========================================================================
// Errors
// ===========================================================================

/// Errors that can occur during data loading.
#[derive(Debug, thiserror::Error)]
pub enum DataLoadError {
    /// A required data file was not found in the given directory.
    #[error("required file '{file}' not found in {dir}")]
    MissingRequired { file: &'static str, dir: PathBuf },

    /// The file has an extension we don't support.
    #[error("unsupported format for file: {file}")]
    UnsupportedFormat { file: PathBuf },

    /// Two files with the same base name but different formats exist.
    #[error("conflicting formats: {a} and {b}")]
    ConflictingFormats { a: PathBuf, b: PathBuf },

    /// A deserialization error occurred.
    #[error("parse error in {file}: {detail}")]
    Parse { file: PathBuf, detail: String },

    /// A name reference could not be resolved.
    #[error("unresolved {expected_kind} reference '{name}' in {file}")]
    UnresolvedRef {
        file: PathBuf,
        name: String,
        expected_kind: &'static str,
    },

    /// A duplicate name was found.
    #[error("duplicate name '{name}' in {file}")]
    DuplicateName { file: PathBuf, name: String },

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ===========================================================================
// Format detection
// ===========================================================================

/// Supported data file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ron,
    Toml,
    Json,
}

/// Detect the format of a file based on its extension.
pub fn detect_format(path: &Path) -> Result<Format, DataLoadError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ron") => Ok(Format::Ron),
        Some("toml") => Ok(Format::Toml),
        Some("json") => Ok(Format::Json),
        _ => Err(DataLoadError::UnsupportedFormat {
            file: path.to_path_buf(),
        }),
    }
}

// ===========================================================================
// File discovery
// ===========================================================================

/// Scan a directory for a data file with the given base name (without extension).
///
/// Looks for `{base_name}.ron`, `{base_name}.toml`, and `{base_name}.json`.
/// Returns `Ok(None)` if no file is found, or `Err(ConflictingFormats)` if
/// multiple formats exist for the same base name.
pub fn find_data_file(dir: &Path, base_name: &str) -> Result<Option<PathBuf>, DataLoadError> {
    let extensions = ["ron", "toml", "json"];
    let mut found: Option<PathBuf> = None;

    for ext in &extensions {
        let candidate = dir.join(format!("{base_name}.{ext}"));
        if candidate.exists() {
            if let Some(ref existing) = found {
                return Err(DataLoadError::ConflictingFormats {
                    a: existing.clone(),
                    b: candidate,
                });
            }
            found = Some(candidate);
        }
    }

    Ok(found)
}

/// Like [`find_data_file`], but returns an error if no file is found.
pub fn require_data_file(dir: &Path, base_name: &str) -> Result<PathBuf, DataLoadError> {
    find_data_file(dir, base_name)?.ok_or_else(|| DataLoadError::MissingRequired {
        file: Box::leak(base_name.to_string().into_boxed_str()),
        dir: dir.to_path_buf(),
    })
}

// ===========================================================================
// Deserialization
// ===========================================================================

/// Read a file and deserialize it according to its format (detected from extension).
pub fn deserialize_file<T: DeserializeOwned>(path: &Path) -> Result<T, DataLoadError> {
    let format = detect_format(path)?;
    let content = std::fs::read_to_string(path)?;

    match format {
        Format::Ron => ron::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Json => serde_json::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Toml => toml::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
    }
}

/// Deserialize a list from a file. For TOML files, extracts the array at the
/// given `toml_key` from a top-level table. For RON and JSON, deserializes
/// directly as `Vec<T>`.
pub fn deserialize_list<T: DeserializeOwned>(
    path: &Path,
    toml_key: &str,
) -> Result<Vec<T>, DataLoadError> {
    let format = detect_format(path)?;
    let content = std::fs::read_to_string(path)?;

    match format {
        Format::Ron => ron::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Json => serde_json::from_str(&content).map_err(|e| DataLoadError::Parse {
            file: path.to_path_buf(),
            detail: e.to_string(),
        }),
        Format::Toml => {
            let table: toml::Value =
                toml::from_str(&content).map_err(|e| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: e.to_string(),
                })?;
            let array = table
                .get(toml_key)
                .ok_or_else(|| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: format!("missing key '{toml_key}' in TOML file"),
                })?
                .clone();
            // Deserialize the array value into Vec<T>.
            array
                .try_into()
                .map_err(|e: toml::de::Error| DataLoadError::Parse {
                    file: path.to_path_buf(),
                    detail: e.to_string(),
                })
        }
    }
}

// ===========================================================================
// Name resolution helpers
// ===========================================================================

/// Look up a name in a map, returning an `UnresolvedRef` error if not found.
pub fn resolve_name<'a, V>(
    map: &'a HashMap<String, V>,
    name: &str,
    file: &Path,
    expected_kind: &'static str,
) -> Result<&'a V, DataLoadError> {
    map.get(name).ok_or_else(|| DataLoadError::UnresolvedRef {
        file: file.to_path_buf(),
        name: name.to_string(),
        expected_kind,
    })
}

/// Check whether a name already exists in a map, returning a `DuplicateName`
/// error if so.
pub fn check_duplicate<V>(
    map: &HashMap<String, V>,
    name: &str,
    file: &Path,
) -> Result<(), DataLoadError> {
    if map.contains_key(name) {
        Err(DataLoadError::DuplicateName {
            file: file.to_path_buf(),
            name: name.to_string(),
        })
    } else {
        Ok(())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary directory with a unique name for test isolation.
    fn make_test_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "factorial_data_test_{suffix}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Clean up a test directory.
    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // -----------------------------------------------------------------------
    // detect_format
    // -----------------------------------------------------------------------

    #[test]
    fn detect_format_ron() {
        assert_eq!(detect_format(Path::new("items.ron")).unwrap(), Format::Ron);
    }

    #[test]
    fn detect_format_toml() {
        assert_eq!(
            detect_format(Path::new("items.toml")).unwrap(),
            Format::Toml
        );
    }

    #[test]
    fn detect_format_json() {
        assert_eq!(
            detect_format(Path::new("items.json")).unwrap(),
            Format::Json
        );
    }

    #[test]
    fn detect_format_unsupported() {
        let result = detect_format(Path::new("items.yaml"));
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));
    }

    #[test]
    fn detect_format_no_extension() {
        let result = detect_format(Path::new("items"));
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));
    }

    // -----------------------------------------------------------------------
    // find_data_file
    // -----------------------------------------------------------------------

    #[test]
    fn find_data_file_found_ron() {
        let dir = make_test_dir("find_ron");
        fs::write(dir.join("items.ron"), "[]").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.ron")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_found_json() {
        let dir = make_test_dir("find_json");
        fs::write(dir.join("items.json"), "[]").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.json")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_found_toml() {
        let dir = make_test_dir("find_toml");
        fs::write(dir.join("items.toml"), "").unwrap();

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, Some(dir.join("items.toml")));

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_missing() {
        let dir = make_test_dir("find_missing");

        let result = find_data_file(&dir, "items").unwrap();
        assert_eq!(result, None);

        cleanup(&dir);
    }

    #[test]
    fn find_data_file_conflict() {
        let dir = make_test_dir("find_conflict");
        fs::write(dir.join("items.ron"), "[]").unwrap();
        fs::write(dir.join("items.json"), "[]").unwrap();

        let result = find_data_file(&dir, "items");
        assert!(matches!(
            result,
            Err(DataLoadError::ConflictingFormats { .. })
        ));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // require_data_file
    // -----------------------------------------------------------------------

    #[test]
    fn require_data_file_found() {
        let dir = make_test_dir("require_found");
        fs::write(dir.join("items.ron"), "[]").unwrap();

        let result = require_data_file(&dir, "items").unwrap();
        assert_eq!(result, dir.join("items.ron"));

        cleanup(&dir);
    }

    #[test]
    fn require_data_file_missing() {
        let dir = make_test_dir("require_missing");

        let result = require_data_file(&dir, "items");
        assert!(matches!(result, Err(DataLoadError::MissingRequired { .. })));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // deserialize_file
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_file_ron() {
        let dir = make_test_dir("deser_ron");
        let path = dir.join("items.ron");
        fs::write(&path, r#"[(name: "iron_ore"), (name: "copper_ore")]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_file(&path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_json() {
        let dir = make_test_dir("deser_json");
        let path = dir.join("items.json");
        fs::write(&path, r#"[{"name": "iron_ore"}, {"name": "copper_ore"}]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_file(&path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_toml() {
        let dir = make_test_dir("deser_toml");
        let path = dir.join("items.toml");
        fs::write(
            &path,
            r#"
[[items]]
name = "iron_ore"

[[items]]
name = "copper_ore"
"#,
        )
        .unwrap();

        let wrapper: crate::schema::TomlItems = deserialize_file(&path).unwrap();
        assert_eq!(wrapper.items.len(), 2);
        assert_eq!(wrapper.items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_parse_error() {
        let dir = make_test_dir("deser_parse_err");
        let path = dir.join("bad.ron");
        fs::write(&path, "this is not valid RON {{{").unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_file(&path);
        assert!(matches!(result, Err(DataLoadError::Parse { .. })));

        cleanup(&dir);
    }

    #[test]
    fn deserialize_file_unsupported_format() {
        let dir = make_test_dir("deser_unsupported");
        let path = dir.join("items.yaml");
        fs::write(&path, "").unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_file(&path);
        assert!(matches!(
            result,
            Err(DataLoadError::UnsupportedFormat { .. })
        ));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // deserialize_list
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_list_ron() {
        let dir = make_test_dir("list_ron");
        let path = dir.join("items.ron");
        fs::write(&path, r#"[(name: "iron_ore"), (name: "copper_ore")]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_json() {
        let dir = make_test_dir("list_json");
        let path = dir.join("items.json");
        fs::write(&path, r#"[{"name": "iron_ore"}, {"name": "copper_ore"}]"#).unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_toml() {
        let dir = make_test_dir("list_toml");
        let path = dir.join("items.toml");
        fs::write(
            &path,
            r#"
[[items]]
name = "iron_ore"

[[items]]
name = "copper_ore"
"#,
        )
        .unwrap();

        let items: Vec<crate::schema::ItemData> = deserialize_list(&path, "items").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "iron_ore");

        cleanup(&dir);
    }

    #[test]
    fn deserialize_list_toml_missing_key() {
        let dir = make_test_dir("list_toml_missing");
        let path = dir.join("items.toml");
        fs::write(&path, r#"foo = "bar""#).unwrap();

        let result: Result<Vec<crate::schema::ItemData>, _> = deserialize_list(&path, "items");
        assert!(matches!(result, Err(DataLoadError::Parse { .. })));

        cleanup(&dir);
    }

    // -----------------------------------------------------------------------
    // resolve_name / check_duplicate
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_name_found() {
        let mut map = HashMap::new();
        map.insert("iron_ore".to_string(), 42u32);

        let val = resolve_name(&map, "iron_ore", Path::new("items.ron"), "item").unwrap();
        assert_eq!(*val, 42);
    }

    #[test]
    fn resolve_name_missing() {
        let map: HashMap<String, u32> = HashMap::new();

        let result = resolve_name(&map, "iron_ore", Path::new("items.ron"), "item");
        assert!(matches!(
            result,
            Err(DataLoadError::UnresolvedRef { ref name, expected_kind: "item", .. }) if name == "iron_ore"
        ));
    }

    #[test]
    fn check_duplicate_no_dup() {
        let map: HashMap<String, u32> = HashMap::new();
        assert!(check_duplicate(&map, "iron_ore", Path::new("items.ron")).is_ok());
    }

    #[test]
    fn check_duplicate_has_dup() {
        let mut map = HashMap::new();
        map.insert("iron_ore".to_string(), 42u32);

        let result = check_duplicate(&map, "iron_ore", Path::new("items.ron"));
        assert!(matches!(
            result,
            Err(DataLoadError::DuplicateName { ref name, .. }) if name == "iron_ore"
        ));
    }

    // -----------------------------------------------------------------------
    // Error display messages
    // -----------------------------------------------------------------------

    #[test]
    fn error_display_messages() {
        let e = DataLoadError::MissingRequired {
            file: "items",
            dir: PathBuf::from("/data"),
        };
        assert!(format!("{e}").contains("items"));
        assert!(format!("{e}").contains("/data"));

        let e = DataLoadError::UnsupportedFormat {
            file: PathBuf::from("items.yaml"),
        };
        assert!(format!("{e}").contains("items.yaml"));

        let e = DataLoadError::ConflictingFormats {
            a: PathBuf::from("items.ron"),
            b: PathBuf::from("items.json"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("items.ron"));
        assert!(msg.contains("items.json"));

        let e = DataLoadError::Parse {
            file: PathBuf::from("bad.ron"),
            detail: "syntax error".to_string(),
        };
        assert!(format!("{e}").contains("bad.ron"));
        assert!(format!("{e}").contains("syntax error"));

        let e = DataLoadError::UnresolvedRef {
            file: PathBuf::from("buildings.ron"),
            name: "iron_ore".to_string(),
            expected_kind: "item",
        };
        let msg = format!("{e}");
        assert!(msg.contains("iron_ore"));
        assert!(msg.contains("item"));

        let e = DataLoadError::DuplicateName {
            file: PathBuf::from("items.ron"),
            name: "iron_ore".to_string(),
        };
        assert!(format!("{e}").contains("iron_ore"));
    }

    // -----------------------------------------------------------------------
    // Io error conversion
    // -----------------------------------------------------------------------

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let data_err: DataLoadError = io_err.into();
        assert!(matches!(data_err, DataLoadError::Io(_)));
        assert!(format!("{data_err}").contains("file not found"));
    }
}
