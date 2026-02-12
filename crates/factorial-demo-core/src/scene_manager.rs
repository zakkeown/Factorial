use std::path::{Path, PathBuf};

use factorial_core::id::EdgeId;
use factorial_core::query::{NodeSnapshot, TransportSnapshot};

use crate::error::DemoError;
use crate::manifest::{SceneEntry, SceneManifest, TierDef, load_manifest};
use crate::scene_builder::{ActiveScene, EdgeMeta, NodeMeta, build_scene};
use crate::scene_schema::SceneData;

pub use factorial_tech_tree::TechTree;

/// Manages the demo showcase: loads the manifest, loads/unloads scenes,
/// drives simulation ticks, and provides query access.
pub struct SceneManager {
    scenes_dir: PathBuf,
    manifest: SceneManifest,
    active: Option<ActiveScene>,
}

impl SceneManager {
    /// Create a new SceneManager by loading the manifest from `scenes_dir`.
    pub fn new(scenes_dir: &Path) -> Result<Self, DemoError> {
        let manifest = load_manifest(scenes_dir)?;
        Ok(Self {
            scenes_dir: scenes_dir.to_path_buf(),
            manifest,
            active: None,
        })
    }

    /// The tier definitions from the manifest.
    pub fn tiers(&self) -> &[TierDef] {
        &self.manifest.tiers
    }

    /// All scene entries from the manifest.
    pub fn scenes(&self) -> &[SceneEntry] {
        &self.manifest.scenes
    }

    /// Scene entries filtered to a specific tier.
    pub fn scenes_in_tier(&self, tier: u8) -> Vec<&SceneEntry> {
        self.manifest
            .scenes
            .iter()
            .filter(|s| s.tier == tier)
            .collect()
    }

    /// Load a scene by its manifest ID. Unloads any previously active scene.
    pub fn load_scene(&mut self, scene_id: &str) -> Result<(), DemoError> {
        let entry = self
            .manifest
            .scenes
            .iter()
            .find(|s| s.id == scene_id)
            .ok_or_else(|| DemoError::SceneNotFound {
                id: scene_id.to_string(),
            })?;

        let scene_dir = self.scenes_dir.join(&entry.path);
        let scene = build_scene(&scene_dir)?;
        self.active = Some(scene);
        Ok(())
    }

    /// Unload the current scene.
    pub fn unload_scene(&mut self) {
        self.active = None;
    }

    /// Advance the simulation by one tick (engine + modules).
    pub fn tick(&mut self) -> Result<(), DemoError> {
        let scene = self.active.as_mut().ok_or(DemoError::NoActiveScene)?;
        scene.engine.step();
        let tick = scene.engine.sim_state.tick;
        if let Some(ref mut pm) = scene.power_module {
            pm.tick(tick);
        }
        if let Some(ref mut fm) = scene.fluid_module {
            fm.tick(tick);
        }
        Ok(())
    }

    /// Advance the simulation by `n` ticks.
    pub fn tick_n(&mut self, n: u64) -> Result<(), DemoError> {
        let scene = self.active.as_mut().ok_or(DemoError::NoActiveScene)?;
        for _ in 0..n {
            scene.engine.step();
            let tick = scene.engine.sim_state.tick;
            if let Some(ref mut pm) = scene.power_module {
                pm.tick(tick);
            }
            if let Some(ref mut fm) = scene.fluid_module {
                fm.tick(tick);
            }
        }
        Ok(())
    }

    /// Set paused state.
    pub fn set_paused(&mut self, paused: bool) -> Result<(), DemoError> {
        let scene = self.active.as_mut().ok_or(DemoError::NoActiveScene)?;
        if paused {
            scene.engine.pause();
        } else {
            scene.engine.resume();
        }
        scene.paused = paused;
        Ok(())
    }

    /// Get the current tick count.
    pub fn current_tick(&self) -> Result<u64, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(scene.engine.sim_state.tick)
    }

    /// Snapshot all nodes in the active scene.
    pub fn snapshot_all_nodes(&self) -> Result<Vec<NodeSnapshot>, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(scene.engine.snapshot_all_nodes())
    }

    /// Snapshot a single node by its scene ID.
    pub fn snapshot_node(&self, scene_node_id: &str) -> Result<Option<NodeSnapshot>, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        let node_id = match scene.node_id_map.get(scene_node_id) {
            Some(id) => *id,
            None => return Ok(None),
        };
        Ok(scene.engine.snapshot_node(node_id))
    }

    /// Snapshot a transport edge.
    pub fn snapshot_transport(
        &self,
        edge_id: EdgeId,
    ) -> Result<Option<TransportSnapshot>, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(scene.engine.snapshot_transport(edge_id))
    }

    /// Node metadata for the active scene.
    pub fn node_meta(&self) -> Result<&[NodeMeta], DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(&scene.node_meta)
    }

    /// Edge metadata for the active scene.
    pub fn edge_meta(&self) -> Result<&[EdgeMeta], DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(&scene.edge_meta)
    }

    /// The active scene's SceneData.
    pub fn active_scene_data(&self) -> Result<&SceneData, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(&scene.scene_data)
    }

    /// The deterministic state hash of the active scene's engine.
    pub fn state_hash(&self) -> Result<u64, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        Ok(scene.engine.state_hash())
    }

    /// Look up an item's name by its type ID, using the engine's registry.
    pub fn item_name(&self, id: factorial_core::id::ItemTypeId) -> Option<String> {
        let scene = self.active.as_ref()?;
        let registry = scene.engine.registry()?;
        registry.get_item(id).map(|def| def.name.clone())
    }

    /// Gallery title from the manifest.
    pub fn gallery_title(&self) -> &str {
        &self.manifest.gallery_title
    }

    /// Gallery description from the manifest.
    pub fn gallery_description(&self) -> &str {
        &self.manifest.gallery_description
    }

    // --- Module queries ---

    /// Power satisfaction for a named network (0.0 to 1.0), or None if unavailable.
    pub fn power_satisfaction(&self, network_name: &str) -> Option<f64> {
        let scene = self.active.as_ref()?;
        let pm = scene.power_module.as_ref()?;
        let net_id = scene.power_network_names.get(network_name)?;
        pm.satisfaction(*net_id).map(|f| f.to_num::<f64>())
    }

    /// Fluid pressure for a named network (0.0 to 1.0), or None if unavailable.
    pub fn fluid_pressure(&self, network_name: &str) -> Option<f64> {
        let scene = self.active.as_ref()?;
        let fm = scene.fluid_module.as_ref()?;
        let net_id = scene.fluid_network_names.get(network_name)?;
        fm.pressure(*net_id).map(|f| f.to_num::<f64>())
    }

    /// Whether a logic circuit control on a scene node is active.
    pub fn logic_is_active(&self, scene_node_id: &str) -> Option<bool> {
        let scene = self.active.as_ref()?;
        let node_id = scene.node_id_map.get(scene_node_id)?;
        let bridge = scene
            .engine
            .find_module::<factorial_logic::LogicModuleBridge>()?;
        bridge.logic().is_active(*node_id)
    }

    /// Access the tech tree if the active scene has one.
    pub fn tech_tree(&self) -> Option<&TechTree> {
        self.active.as_ref()?.tech_tree.as_ref()
    }

    /// Whether the active scene has a power module.
    pub fn has_power(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|s| s.power_module.is_some())
    }

    /// Whether the active scene has a fluid module.
    pub fn has_fluid(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|s| s.fluid_module.is_some())
    }

    /// Serialize the active engine's state (for save/load demos).
    pub fn serialize_engine(&self) -> Result<Vec<u8>, DemoError> {
        let scene = self.active.as_ref().ok_or(DemoError::NoActiveScene)?;
        scene
            .engine
            .serialize()
            .map_err(|e| DemoError::Serialization {
                detail: e.to_string(),
            })
    }
}
