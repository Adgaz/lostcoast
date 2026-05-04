use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct ProbeObb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

pub fn parallax_corrected_dir(probe: &ProbeObb, surface: Vec3, reflect_dir: Vec3) -> Vec3 {
    let inv = Vec3::ONE / reflect_dir;
    let pos_t = (probe.center + probe.half_extents - surface) * inv;
    let neg_t = (probe.center - probe.half_extents - surface) * inv;
    let further = pos_t.max(neg_t);
    let t = further.x.min(further.y).min(further.z).max(0.0);
    let hit = surface + reflect_dir * t;
    (hit - probe.center).normalize()
}

pub fn nearest_dir(reflect_dir: Vec3) -> Vec3 {
    reflect_dir.normalize()
}
