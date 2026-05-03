use anyhow::Result;
use clap::Parser;
use image::DynamicImage;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(about = "ssim + max-delta png comparator")]
struct Args {
    #[arg(long)]
    reference: PathBuf,
    #[arg(long)]
    actual: PathBuf,
    #[arg(long, default_value_t = 0.995)]
    ssim_min: f64,
    #[arg(long, default_value_t = 4)]
    max_delta: u8,
}

fn load(p: &PathBuf) -> Result<image::RgbaImage> {
    let img = image::open(p)?;
    Ok(match img {
        DynamicImage::ImageRgba8(i) => i,
        other => other.to_rgba8(),
    })
}

fn main() -> ExitCode {
    let args = Args::parse();
    let r = match load(&args.reference) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("reference load failed: {e}");
            return ExitCode::from(4);
        }
    };
    let a = match load(&args.actual) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("actual load failed: {e}");
            return ExitCode::from(4);
        }
    };
    if r.dimensions() != a.dimensions() {
        eprintln!(
            "size mismatch: ref {:?} vs actual {:?}",
            r.dimensions(),
            a.dimensions()
        );
        return ExitCode::from(3);
    }
    let s = harness::ssim(&r, &a).unwrap();
    let d = harness::max_delta(&r, &a).unwrap();
    println!("ssim={s:.6} max_delta={d}");
    if s < args.ssim_min {
        eprintln!("STAGE_FAIL: ssim {s:.6} < {}", args.ssim_min);
        return ExitCode::from(1);
    }
    if d > args.max_delta {
        eprintln!("STAGE_FAIL: delta {d} > {}", args.max_delta);
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
