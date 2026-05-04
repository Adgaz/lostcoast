use glam::Vec3;

use crate::bvh::{Bvh, Ray};

pub const RAY_BIAS: f32 = 1e-4;

pub fn visible(bvh: &Bvh, a: Vec3, b: Vec3, na: Vec3, nb: Vec3) -> bool {
    let dir = b - a;
    let dist = dir.length();
    if dist < 1e-6 {
        return true;
    }
    let dirn = dir / dist;
    let origin = a + na * RAY_BIAS;
    let target = b + nb * RAY_BIAS;
    let span = (target - origin).length();
    let ray = Ray {
        origin,
        direction: dirn,
        tmax: span - RAY_BIAS,
    };
    !bvh.occluded(&ray)
}
