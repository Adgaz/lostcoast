use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "headless offscreen render for screenshot regression gates")]
struct Args {
    #[arg(long)]
    scene: PathBuf,
    #[arg(long, value_parser = parse_vec3)]
    camera: [f32; 3],
    #[arg(long, value_parser = parse_vec3, default_value = "0,0,0")]
    look_at: [f32; 3],
    #[arg(long, default_value = "1280x720")]
    size: String,
    #[arg(long)]
    out: PathBuf,
}

fn parse_vec3(s: &str) -> Result<[f32; 3], String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err(format!("expected x,y,z, got {s}"));
    }
    let parse = |p: &str| p.trim().parse::<f32>().map_err(|e| e.to_string());
    Ok([parse(parts[0])?, parse(parts[1])?, parse(parts[2])?])
}

fn parse_size(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.split(['x', 'X']).collect();
    if parts.len() != 2 {
        anyhow::bail!("expected WxH, got {s}");
    }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let size = parse_size(&args.size)?;
    let img = harness::render_scene(&args.scene, args.camera, args.look_at, size)?;
    img.save(&args.out)?;
    Ok(())
}
