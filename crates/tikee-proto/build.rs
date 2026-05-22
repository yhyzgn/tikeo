//! Build script for generated tikee protobuf bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost_config = tonic_prost_build::Config::new();
    prost_config.protoc_executable(protoc);
    prost_config.boxed(".tikee.worker.v1.ServerMessage.Kind.dispatch_task");
    prost_config.boxed(".tikee.worker.v1.ServerMessage.dispatch_task");
    prost_config.boxed(".tikee.worker.v1.DispatchTask.processor_binding");

    tonic_prost_build::configure()
        .build_client(true)
        .compile_with_config(prost_config, &["proto/worker.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/worker.proto");
    Ok(())
}
