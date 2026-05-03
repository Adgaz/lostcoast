use anyhow::{Context, Result};
use ash::vk;

pub const TRIANGLE_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));
pub const TRIANGLE_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
pub const WORLD_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/world.vert.spv"));
pub const WORLD_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/world.frag.spv"));

pub fn load_module(device: &ash::Device, spv: &[u8]) -> Result<vk::ShaderModule> {
    if spv.len() % 4 != 0 {
        anyhow::bail!("SPV not multiple of 4 bytes: {}", spv.len());
    }
    let words: Vec<u32> = spv
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let info = vk::ShaderModuleCreateInfo::default().code(&words);
    unsafe { device.create_shader_module(&info, None) }.context("create shader module")
}
