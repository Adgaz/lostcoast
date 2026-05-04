use glam::Vec3;

pub const B0: Vec3 = Vec3::new(-0.408_248_29, 0.707_106_77, 0.577_350_26);
pub const B1: Vec3 = Vec3::new(-0.408_248_29, -0.707_106_77, 0.577_350_26);
pub const B2: Vec3 = Vec3::new(0.816_496_6, 0.0, 0.577_350_26);

pub const ALL: [Vec3; 3] = [B0, B1, B2];

pub fn compose(n: Vec3, lm: [Vec3; 3]) -> Vec3 {
    let w0 = n.dot(B0).max(0.0);
    let w1 = n.dot(B1).max(0.0);
    let w2 = n.dot(B2).max(0.0);
    lm[0] * w0 + lm[1] * w1 + lm[2] * w2
}

pub fn project(direction: Vec3, value: Vec3) -> [Vec3; 3] {
    let w0 = direction.dot(B0).max(0.0);
    let w1 = direction.dot(B1).max(0.0);
    let w2 = direction.dot(B2).max(0.0);
    [value * w0, value * w1, value * w2]
}
