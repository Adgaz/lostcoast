use glam::Vec3;

use crate::bvh::{Bvh, Ray};
use crate::scene::{BakeScene, BakeTriangle};

pub struct Settings {
    pub samples_per_patch: u32,
    pub max_iters: u32,
    pub seed: u64,
    pub epsilon: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            samples_per_patch: 4096,
            max_iters: 64,
            seed: 0xc0ffee_u64,
            epsilon: 1e-4,
        }
    }
}

pub struct Solution {
    pub radiosity: Vec<Vec3>,
    pub iterations: u32,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let r = self.0;
        let r = (r ^ (r >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
        let r = (r ^ (r >> 33)).wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        r ^ (r >> 33)
    }
    fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        bits as f32 * (1.0 / (1 << 24) as f32)
    }
}

fn cosine_hemisphere(rng: &mut Rng) -> Vec3 {
    let u1 = rng.next_f32();
    let u2 = rng.next_f32();
    let r = u1.sqrt();
    let phi = 2.0 * std::f32::consts::PI * u2;
    let x = r * phi.cos();
    let y = r * phi.sin();
    let z = (1.0 - u1).max(0.0).sqrt();
    Vec3::new(x, y, z)
}

fn sample_triangle(tri: &BakeTriangle, rng: &mut Rng) -> Vec3 {
    let mut u = rng.next_f32();
    let mut v = rng.next_f32();
    if u + v > 1.0 {
        u = 1.0 - u;
        v = 1.0 - v;
    }
    tri.v0 + (tri.v1 - tri.v0) * u + (tri.v2 - tri.v0) * v
}

fn build_tbn(n: Vec3) -> (Vec3, Vec3) {
    let s = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = Vec3::new(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = Vec3::new(b, s + n.y * n.y * a, -n.y);
    (t, bt)
}

pub fn solve(scene: &BakeScene, settings: Settings) -> Solution {
    let triangles = scene.flatten();
    let bvh = Bvh::build(triangles.clone());
    let n = triangles.len();
    let mesh_albedo: Vec<Vec3> = scene.meshes.iter().map(|m| m.albedo).collect();
    let mesh_emissive: Vec<Vec3> = scene.meshes.iter().map(|m| m.emissive).collect();

    let normals: Vec<Vec3> = triangles.iter().map(|t| t.normal()).collect();
    let mut form_rows: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n];

    let inv_samples = 1.0 / settings.samples_per_patch as f32;
    for i in 0..n {
        let mut rng = Rng::new(settings.seed.wrapping_add(i as u64 * 0x9E37_79B9));
        let ni = normals[i];
        let (t, b) = build_tbn(ni);
        let mut hits: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for _ in 0..settings.samples_per_patch {
            let origin = sample_triangle(&triangles[i], &mut rng);
            let local = cosine_hemisphere(&mut rng);
            let dir = (t * local.x + b * local.y + ni * local.z).normalize();
            let ray = Ray {
                origin: origin + ni * 1e-4,
                direction: dir,
                tmax: 1e6,
            };
            if let Some(hit) = bvh.closest_hit(&ray) {
                *hits.entry(hit.triangle).or_insert(0) += 1;
            }
        }
        let mut row = Vec::with_capacity(hits.len());
        for (j, count) in hits {
            row.push((j, count as f32 * inv_samples));
        }
        form_rows[i] = row;
    }

    let emit: Vec<Vec3> = triangles
        .iter()
        .map(|t| mesh_emissive[t.mesh as usize])
        .collect();
    let albedo: Vec<Vec3> = triangles
        .iter()
        .map(|t| mesh_albedo[t.mesh as usize])
        .collect();

    let mut radiosity: Vec<Vec3> = emit.clone();
    let mut iterations = 0;
    for it in 0..settings.max_iters {
        iterations = it + 1;
        let mut next: Vec<Vec3> = emit.clone();
        for i in 0..n {
            let mut acc = Vec3::ZERO;
            for &(j, f) in &form_rows[i] {
                acc += radiosity[j as usize] * f;
            }
            next[i] += albedo[i] * acc;
        }
        let mut max_delta = 0.0_f32;
        for i in 0..n {
            let d = (next[i] - radiosity[i]).abs().max_element();
            if d > max_delta {
                max_delta = d;
            }
        }
        radiosity = next;
        if max_delta < settings.epsilon {
            break;
        }
    }

    Solution {
        radiosity,
        iterations,
    }
}

pub fn total_flux(scene: &BakeScene, solution: &Solution) -> Vec3 {
    let triangles = scene.flatten();
    let mut sum = Vec3::ZERO;
    for (i, t) in triangles.iter().enumerate() {
        sum += solution.radiosity[i] * t.area();
    }
    sum
}
