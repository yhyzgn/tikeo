//! Build script for standalone Rust Worker SDK protobuf bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost_config = tonic_prost_build::Config::new();
    prost_config.protoc_executable(protoc);

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_with_config(prost_config, &["proto/worker.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/worker.proto");
    Ok(())
}
