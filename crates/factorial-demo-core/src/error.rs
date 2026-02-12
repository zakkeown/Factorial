use std::path::PathBuf;

/// Errors that can occur in the demo showcase.
#[derive(Debug, thiserror::Error)]
pub enum DemoError {
    /// No scene is currently loaded.
    #[error("no scene is currently loaded")]
    NoActiveScene,

    /// The requested scene was not found in the manifest.
    #[error("scene '{id}' not found in manifest")]
    SceneNotFound { id: String },

    /// A scene node ID referenced in an edge was not found.
    #[error("scene node '{id}' not found (referenced by edge)")]
    NodeNotFound { id: String },

    /// A building name in scene.ron does not match buildings.ron.
    #[error("building '{name}' not found in game data")]
    BuildingNotFound { name: String },

    /// An item name in scene.ron does not match items.ron.
    #[error("item '{name}' not found in game data")]
    ItemNotFound { name: String },

    /// Failed to load game data from the scene directory.
    #[error("data load error in {dir}: {source}")]
    DataLoad {
        dir: PathBuf,
        source: factorial_data::DataLoadError,
    },

    /// Failed to parse a scene or manifest file.
    #[error("parse error in {file}: {detail}")]
    Parse { file: PathBuf, detail: String },

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A graph mutation failed to resolve.
    #[error("graph mutation failed: {detail}")]
    GraphMutation { detail: String },

    /// Module wiring failed.
    #[error("module wiring error: {detail}")]
    ModuleWiring { detail: String },

    /// Serialization error.
    #[error("serialization error: {detail}")]
    Serialization { detail: String },
}
