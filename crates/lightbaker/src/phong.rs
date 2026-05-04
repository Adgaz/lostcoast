use glam::Vec3;

pub fn reflect(l: Vec3, n: Vec3) -> Vec3 {
    (n * (2.0 * n.dot(l)) - l).normalize()
}

pub fn specular_align(light_dir: Vec3, view_dir: Vec3, normal: Vec3) -> f32 {
    let r = reflect(light_dir, normal);
    r.dot(view_dir)
}

pub fn fresnel_remap(ranges: [f32; 3], n_dot_v: f32) -> f32 {
    let [a, b, c] = ranges;
    let mid = b;
    if n_dot_v <= mid {
        let t = ((n_dot_v - 0.0) / mid.max(1e-6)).clamp(0.0, 1.0);
        a + (mid - a) * t * t
    } else {
        let t = ((n_dot_v - mid) / (1.0 - mid).max(1e-6)).clamp(0.0, 1.0);
        mid + (c - mid) * t * t
    }
}

pub fn phong_specular(
    light_dir: Vec3,
    view_dir: Vec3,
    normal: Vec3,
    exponent: f32,
    boost: f32,
    fresnel: f32,
) -> f32 {
    let r = reflect(light_dir, normal);
    let rv = r.dot(view_dir).max(0.0);
    rv.powf(exponent) * boost * fresnel
}
