fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Vendored protoc keeps the build hermetic; no system protoc needed.
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    tonic_build::configure()
        .build_server(false)
        .compile_protos(&["proto/homelab/agent/v1/agent.proto"], &["proto"])?;
    Ok(())
}
