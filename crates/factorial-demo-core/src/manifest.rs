use serde::Deserialize;
use std::path::Path;

use crate::error::DemoError;

/// Top-level manifest listing all demo scenes.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneManifest {
    pub gallery_title: String,
    pub gallery_description: String,
    #[serde(default)]
    pub tiers: Vec<TierDef>,
    pub scenes: Vec<SceneEntry>,
}

/// Tier definition for grouping scenes.
#[derive(Debug, Clone, Deserialize)]
pub struct TierDef {
    pub number: u8,
    pub name: String,
    pub description: String,
}

/// An entry in the manifest pointing to a scene directory.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tier: u8,
    /// Relative path from the scenes directory to the scene directory.
    pub path: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires_modules: Vec<String>,
}

/// Load the scene manifest from a `manifest.ron` file.
pub fn load_manifest(scenes_dir: &Path) -> Result<SceneManifest, DemoError> {
    let path = scenes_dir.join("manifest.ron");
    let content = std::fs::read_to_string(&path)?;
    ron::from_str(&content).map_err(|e| DemoError::Parse {
        file: path,
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_manifest() {
        let input = r#"(
            gallery_title: "Factorial Demo Showcase",
            gallery_description: "Curated factory vignettes demonstrating engine capabilities.",
            tiers: [
                (number: 1, name: "Fundamentals", description: "Basic building blocks."),
            ],
            scenes: [
                (
                    id: "solo_extractor",
                    title: "Solo Extractor",
                    summary: "A single mine producing iron ore.",
                    tier: 1,
                    path: "tier1_fundamentals/01_solo_extractor",
                ),
            ],
        )"#;

        let manifest: SceneManifest = ron::from_str(input).unwrap();
        assert_eq!(manifest.gallery_title, "Factorial Demo Showcase");
        assert_eq!(manifest.tiers.len(), 1);
        assert_eq!(manifest.tiers[0].number, 1);
        assert_eq!(manifest.scenes.len(), 1);
        assert_eq!(manifest.scenes[0].id, "solo_extractor");
        assert_eq!(
            manifest.scenes[0].path,
            "tier1_fundamentals/01_solo_extractor"
        );
    }

    #[test]
    fn load_manifest_from_file() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenes");
        let manifest = load_manifest(&dir).unwrap();
        assert!(!manifest.gallery_title.is_empty());
        assert!(!manifest.scenes.is_empty());
    }
}
