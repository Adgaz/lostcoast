use glam::Vec3;
use lostcoast_core::rnm_basis;

pub fn project_irradiance(direction: Vec3, value: Vec3) -> [Vec3; 3] {
    rnm_basis::project(direction, value)
}

pub struct RnmTexel {
    pub rnm: [Vec3; 3],
}

pub fn flat_plane_overhead_texel() -> RnmTexel {
    let l_tangent = Vec3::Z;
    let li = Vec3::ONE;
    let weights = [
        rnm_basis::B0.dot(l_tangent).max(0.0),
        rnm_basis::B1.dot(l_tangent).max(0.0),
        rnm_basis::B2.dot(l_tangent).max(0.0),
    ];
    RnmTexel {
        rnm: [li * weights[0], li * weights[1], li * weights[2]],
    }
}
