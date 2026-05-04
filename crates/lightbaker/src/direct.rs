use glam::Vec3;

use crate::bvh::{Bvh, Ray};
use crate::raycast::RAY_BIAS;
use crate::scene::BakeScene;

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    pub position: Vec3,
    pub intensity: Vec3,
}

pub fn direct_radiance(bvh: &Bvh, point: Vec3, normal: Vec3, light: &PointLight) -> Vec3 {
    let to_light = light.position - point;
    let dist = to_light.length();
    if dist < 1e-6 {
        return Vec3::ZERO;
    }
    let dir = to_light / dist;
    let cos_theta = normal.dot(dir);
    if cos_theta <= 0.0 {
        return Vec3::ZERO;
    }
    let ray = Ray {
        origin: point + normal * RAY_BIAS,
        direction: dir,
        tmax: dist - RAY_BIAS,
    };
    if bvh.occluded(&ray) {
        return Vec3::ZERO;
    }
    light.intensity * (cos_theta / (std::f32::consts::PI * dist * dist))
}

pub fn floor_below_light(scene: &BakeScene, light: &PointLight, sample_point: Vec3) -> Vec3 {
    let triangles = scene.flatten();
    let bvh = Bvh::build(triangles);
    direct_radiance(&bvh, sample_point, Vec3::Y, light)
}
