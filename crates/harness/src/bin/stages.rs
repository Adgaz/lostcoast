use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "stage gate config inspector")]
struct Args {
    #[arg(long, default_value = "crates/harness/stages.toml")]
    config: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Show { id: String },
    List,
    Get { id: String, field: String },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let s = harness::load_stages(&args.config)?;
    match args.cmd {
        Cmd::List => {
            for (id, cfg) in &s.stage {
                println!("{id}\t{}\tvisual={}", cfg.bin, cfg.visual_gate);
            }
        }
        Cmd::Show { id } => {
            let cfg = s
                .stage
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("no stage {id}"))?;
            println!("{cfg:#?}");
        }
        Cmd::Get { id, field } => {
            let cfg = s
                .stage
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("no stage {id}"))?;
            // tiny exporter so shell scripts can read fields without jq
            match field.as_str() {
                "bin" => println!("{}", cfg.bin),
                "scene" => println!(
                    "{}",
                    cfg.scene
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ),
                "reference" => println!(
                    "{}",
                    cfg.reference
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ),
                "visual_gate" => println!("{}", cfg.visual_gate),
                "human_review_required" => println!("{}", cfg.human_review_required),
                "ssim_min" => println!("{}", cfg.ssim_min),
                "max_delta" => println!("{}", cfg.max_delta),
                "numerical_tests" => println!("{}", cfg.numerical_tests.join(" ")),
                "camera" => println!(
                    "{}",
                    cfg.camera
                        .map(|c| format!("{},{},{}", c[0], c[1], c[2]))
                        .unwrap_or_default()
                ),
                "look_at" => println!(
                    "{}",
                    cfg.look_at
                        .map(|c| format!("{},{},{}", c[0], c[1], c[2]))
                        .unwrap_or_default()
                ),
                "size" => println!(
                    "{}",
                    cfg.size
                        .map(|s| format!("{}x{}", s[0], s[1]))
                        .unwrap_or_else(|| "1280x720".into())
                ),
                other => anyhow::bail!("unknown field {other}"),
            }
        }
    }
    Ok(())
}
