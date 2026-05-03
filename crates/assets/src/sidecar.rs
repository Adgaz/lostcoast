use anyhow::{Context, Result};
use lostcoast_core::scene::Scene;
use std::path::Path;

pub fn load_scene(path: &Path) -> Result<Scene> {
    let s =
        std::fs::read_to_string(path).with_context(|| format!("read scene {}", path.display()))?;
    let scene: Scene =
        serde_json::from_str(&s).with_context(|| format!("parse scene {}", path.display()))?;
    Ok(scene)
}
