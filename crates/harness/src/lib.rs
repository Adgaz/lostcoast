use anyhow::{anyhow, Result};
use image::RgbaImage;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn render_scene(
    _scene_path: &Path,
    _camera: [f32; 3],
    _look_at: [f32; 3],
    _size: (u32, u32),
) -> Result<RgbaImage> {
    Err(anyhow!(
        "render_scene not wired yet; gfx::render_offscreen needed (stage 1+)"
    ))
}

#[derive(Debug, Deserialize, Clone)]
pub struct Stages {
    pub stage: BTreeMap<String, StageConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StageConfig {
    pub bin: String,
    #[serde(default)]
    pub scene: Option<PathBuf>,
    #[serde(default)]
    pub camera: Option<[f32; 3]>,
    #[serde(default)]
    pub look_at: Option<[f32; 3]>,
    #[serde(default)]
    pub size: Option<[u32; 2]>,
    #[serde(default)]
    pub reference: Option<PathBuf>,
    #[serde(default)]
    pub visual_gate: bool,
    #[serde(default)]
    pub numerical_tests: Vec<String>,
    #[serde(default)]
    pub human_review_required: bool,
    #[serde(default)]
    pub review_note: Option<String>,
    #[serde(default = "default_ssim")]
    pub ssim_min: f64,
    #[serde(default = "default_max_delta")]
    pub max_delta: u8,
}

fn default_ssim() -> f64 {
    0.995
}
fn default_max_delta() -> u8 {
    4
}

pub fn load_stages(path: &Path) -> Result<Stages> {
    let s = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&s)?)
}

pub fn ssim(a: &RgbaImage, b: &RgbaImage) -> Result<f64> {
    if a.dimensions() != b.dimensions() {
        return Err(anyhow!("size mismatch"));
    }
    // simplified single-window ssim per channel, averaged.
    // robust enough as a regression gate; not a perceptual metric.
    let (w, h) = a.dimensions();
    let n = (w * h) as f64;
    let mut sum = 0.0_f64;
    for c in 0..3 {
        let (mut mx, mut my, mut sx, mut sy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (pa, pb) in a.pixels().zip(b.pixels()) {
            let x = pa.0[c] as f64 / 255.0;
            let y = pb.0[c] as f64 / 255.0;
            mx += x;
            my += y;
        }
        mx /= n;
        my /= n;
        for (pa, pb) in a.pixels().zip(b.pixels()) {
            let x = pa.0[c] as f64 / 255.0;
            let y = pb.0[c] as f64 / 255.0;
            sx += (x - mx) * (x - mx);
            sy += (y - my) * (y - my);
            sxy += (x - mx) * (y - my);
        }
        sx /= n - 1.0;
        sy /= n - 1.0;
        sxy /= n - 1.0;
        let c1 = 0.01_f64.powi(2);
        let c2 = 0.03_f64.powi(2);
        let num = (2.0 * mx * my + c1) * (2.0 * sxy + c2);
        let den = (mx * mx + my * my + c1) * (sx + sy + c2);
        sum += num / den;
    }
    Ok(sum / 3.0)
}

pub fn max_delta(a: &RgbaImage, b: &RgbaImage) -> Result<u8> {
    if a.dimensions() != b.dimensions() {
        return Err(anyhow!("size mismatch"));
    }
    let mut m = 0u8;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for c in 0..3 {
            let d = pa.0[c].abs_diff(pb.0[c]);
            if d > m {
                m = d;
            }
        }
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, image::Rgba([rgb[0], rgb[1], rgb[2], 255]))
    }

    #[test]
    fn ssim_identical_is_one() {
        let a = solid(8, 8, [128, 64, 200]);
        let b = a.clone();
        let s = ssim(&a, &b).unwrap();
        assert!(s > 0.999, "ssim={s}");
    }

    #[test]
    fn ssim_mismatch_errors() {
        let a = solid(8, 8, [0, 0, 0]);
        let b = solid(4, 4, [0, 0, 0]);
        assert!(ssim(&a, &b).is_err());
    }

    #[test]
    fn max_delta_zero_for_identical() {
        let a = solid(4, 4, [10, 20, 30]);
        assert_eq!(max_delta(&a, &a).unwrap(), 0);
    }

    #[test]
    fn max_delta_picks_largest() {
        let mut a = solid(2, 1, [0, 0, 0]);
        let b = solid(2, 1, [5, 0, 9]);
        a.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
        assert_eq!(max_delta(&a, &b).unwrap(), 9);
    }
}
