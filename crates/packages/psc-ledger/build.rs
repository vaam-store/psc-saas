fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap(); // Go up three levels to the workspace root
    let proto_root = workspace_root.join("protos");

    println!("Current CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    println!("Resolved WORKSPACE_ROOT: {}", workspace_root.display());
    println!("Resolved PROTO_ROOT for -I: {}", proto_root.display());

    tonic_prost_build::configure()
        .build_server(true)
        .compile_protos(
            &[
                "psc/common/v1/common.proto",   // Relative to proto_root
                "psc/journal/v1/journal.proto", // Relative to proto_root
            ],
            &[proto_root.to_str().unwrap()], // Absolute path as include path
        )?;
    Ok(())
}
