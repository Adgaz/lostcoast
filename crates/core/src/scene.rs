use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scene {
    Clear { clear_color: [f32; 3] },
    Triangle { clear_color: [f32; 3] },
    Cube { clear_color: [f32; 3] },
    Cornell { clear_color: [f32; 3] },
}

impl Scene {
    pub fn clear_color(&self) -> [f32; 3] {
        match self {
            Scene::Clear { clear_color }
            | Scene::Triangle { clear_color }
            | Scene::Cube { clear_color }
            | Scene::Cornell { clear_color } => *clear_color,
        }
    }
}
