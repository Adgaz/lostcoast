use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use lightbaker::radiosity::{solve, total_flux, Settings};
use lightbaker::scene::BakeScene;
use serde::Serialize;

#[derive(Serialize)]
struct Manifest {
    scene: String,
    samples_per_patch: u32,
    bounces: u32,
    seed: u64,
    triangle_count: usize,
    iterations: u32,
    total_flux: [f32; 3],
    radiosity: Vec<[f32; 3]>,
}

fn parse_args() -> Result<(String, PathBuf, u64)> {
    let mut scene = None;
    let mut out = None;
    let mut seed = 0xc0ffee_u64;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scene" => {
                scene = args.get(i + 1).cloned();
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--seed" => {
                seed = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("--seed needs value"))?
                    .parse()
                    .context("parse seed")?;
                i += 2;
            }
            other => return Err(anyhow!("unknown arg {other}")),
        }
    }
    let scene =
        scene.ok_or_else(|| anyhow!("--scene required (closed_box | cornell | flat_plane)"))?;
    let out = out.unwrap_or_else(|| PathBuf::from("bake.json"));
    Ok((scene, out, seed))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let (scene_name, out, seed) = parse_args()?;
    let scene = match scene_name.as_str() {
        "closed_box" => BakeScene::closed_unit_cube(0.5),
        "cornell" => BakeScene::cornell_one_light(),
        "flat_plane" => BakeScene::flat_plane_overhead(),
        other => return Err(anyhow!("unknown scene {other}")),
    };
    let triangles = scene.flatten();
    tracing::info!(
        "baking scene={scene_name} triangles={} seed={seed}",
        triangles.len()
    );
    let settings = Settings {
        samples_per_patch: 4096,
        max_iters: 64,
        seed,
        epsilon: 1e-4,
    };
    let solution = solve(&scene, settings);
    let flux = total_flux(&scene, &solution);
    tracing::info!(
        "iterations={} flux=({:.4},{:.4},{:.4})",
        solution.iterations,
        flux.x,
        flux.y,
        flux.z
    );
    let manifest = Manifest {
        scene: scene_name,
        samples_per_patch: 4096,
        bounces: solution.iterations,
        seed,
        triangle_count: triangles.len(),
        iterations: solution.iterations,
        total_flux: flux.to_array(),
        radiosity: solution.radiosity.iter().map(|v| v.to_array()).collect(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&out, json).with_context(|| format!("write {}", out.display()))?;
    tracing::info!("wrote {}", out.display());
    Ok(())
}
