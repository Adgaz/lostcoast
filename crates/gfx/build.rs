use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let shaders = manifest.join("../../shaders");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let compiler = shaderc::Compiler::new().expect("shaderc compiler");

    let targets = [
        ("triangle.vert", shaderc::ShaderKind::Vertex),
        ("triangle.frag", shaderc::ShaderKind::Fragment),
        ("world.vert", shaderc::ShaderKind::Vertex),
        ("world.frag", shaderc::ShaderKind::Fragment),
    ];

    for (name, kind) in targets {
        let src_path = shaders.join(name);
        println!("cargo:rerun-if-changed={}", src_path.display());
        let src = std::fs::read_to_string(&src_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", src_path.display()));
        let mut opts = shaderc::CompileOptions::new().expect("compile options");
        opts.set_target_env(
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_3 as u32,
        );
        opts.set_source_language(shaderc::SourceLanguage::GLSL);
        let artifact = compiler
            .compile_into_spirv(&src, kind, name, "main", Some(&opts))
            .unwrap_or_else(|e| panic!("compile {name}: {e}"));
        let out_path = out_dir.join(format!("{name}.spv"));
        std::fs::write(&out_path, artifact.as_binary_u8())
            .unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));
    }
}
