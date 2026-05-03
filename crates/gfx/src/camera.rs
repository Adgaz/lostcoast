use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
}

pub fn perspective(aspect: f32) -> Mat4 {
    Mat4::perspective_rh(60f32.to_radians(), aspect, 0.05, 100.0)
}

pub fn view(camera: Vec3, look_at: Vec3) -> Mat4 {
    Mat4::look_at_rh(camera, look_at, Vec3::Y)
}
