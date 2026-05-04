use glam::Vec3;

pub fn half_lambert_from_dot(dot: f32) -> f32 {
    let h = dot * 0.5 + 0.5;
    h * h
}

pub fn half_lambert(n: Vec3, l: Vec3) -> f32 {
    half_lambert_from_dot(n.dot(l))
}

pub fn build_orthonormal(n: Vec3) -> (Vec3, Vec3) {
    let s = n.z.signum();
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = Vec3::new(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = Vec3::new(b, s + n.y * n.y * a, -n.y);
    (t.normalize(), bt.normalize())
}
