//! Build script for generated tikeo protobuf bindings.

use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost_config = tonic_prost_build::Config::new();
    prost_config.protoc_executable(protoc);
    prost_config.boxed(".tikeo.worker.v1.ServerMessage.Kind.dispatch_task");
    prost_config.boxed(".tikeo.worker.v1.ServerMessage.dispatch_task");
    prost_config.boxed(".tikeo.worker.v1.DispatchTask.processor_binding");
    prost_config.type_attribute(
        ".tikeo.worker.v1",
        "#[doc = \"Generated worker protocol item.\"]",
    );
    prost_config.field_attribute(
        ".tikeo.worker.v1",
        "#[doc = \"Generated worker protocol field.\"]",
    );

    tonic_prost_build::configure()
        .build_client(true)
        .compile_with_config(prost_config, &["proto/worker.proto"], &["proto"])?;
    patch_generated_bindings()?;
    println!("cargo:rerun-if-changed=proto/worker.proto");
    Ok(())
}

fn patch_generated_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let generated = PathBuf::from(env::var("OUT_DIR")?).join("tikeo.worker.v1.rs");
    let mut source = fs::read_to_string(&generated)?;
    let replacements = [
        (
            "        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>\n",
            "        /// Connect to a worker tunnel service endpoint.\n        ///\n        /// # Errors\n        ///\n        /// Returns a transport error when the endpoint cannot be converted or connected.\n        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>\n",
        ),
        (
            "        pub async fn open_tunnel(\n",
            "        /// Open a worker tunnel stream.\n        ///\n        /// # Errors\n        ///\n        /// Returns a gRPC status error when the tunnel request fails.\n        pub async fn open_tunnel(\n",
        ),
        (
            "        pub async fn subscribe_task_logs(\n",
            "        /// Subscribe to task log stream.\n        ///\n        /// # Errors\n        ///\n        /// Returns a gRPC status error when the subscription request fails.\n        pub async fn subscribe_task_logs(\n",
        ),
        (
            "Generated trait containing gRPC methods that should be implemented for use with WorkerTunnelServiceServer.",
            "Generated trait containing gRPC methods that should be implemented for use with `WorkerTunnelServiceServer`.",
        ),
        (
            "Server streaming response type for the OpenTunnel method.",
            "Server streaming response type for the `OpenTunnel` method.",
        ),
        (
            "Server streaming response type for the SubscribeTaskLogs method.",
            "Server streaming response type for the `SubscribeTaskLogs` method.",
        ),
        (
            "accept_compression_encodings: Default::default(),",
            "accept_compression_encodings: EnabledCompressionEncodings::default(),",
        ),
        (
            "send_compression_encodings: Default::default(),",
            "send_compression_encodings: EnabledCompressionEncodings::default(),",
        ),
        (
            "        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {\n            self.max_decoding_message_size = Some(limit);",
            "        pub const fn max_decoding_message_size(mut self, limit: usize) -> Self {\n            self.max_decoding_message_size = Some(limit);",
        ),
        (
            "        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {\n            self.max_encoding_message_size = Some(limit);",
            "        pub const fn max_encoding_message_size(mut self, limit: usize) -> Self {\n            self.max_encoding_message_size = Some(limit);",
        ),
    ];
    for (from, to) in replacements {
        source = source.replace(from, to);
    }
    source = add_eq_to_non_float_message_derives(&source);
    fs::write(generated, source)?;
    Ok(())
}

fn add_eq_to_non_float_message_derives(source: &str) -> String {
    let mut output = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "#[derive(Clone, PartialEq, ::prost::Oneof)]" {
            let indent = &line[..line.len() - line.trim_start().len()];
            output.push(format!(
                "{indent}#[derive(Clone, PartialEq, Eq, ::prost::Oneof)]"
            ));
        } else if line.trim() == "#[derive(Clone, PartialEq, ::prost::Message)]" {
            let mut probe = index + 1;
            let mut has_float_field = false;
            while probe < lines.len() {
                let candidate = lines[probe];
                if probe > index + 1
                    && (candidate.starts_with("#[derive(")
                        || candidate.starts_with("pub mod ")
                        || candidate.starts_with("pub struct "))
                {
                    break;
                }
                if candidate.contains("float") || candidate.contains("double") {
                    has_float_field = true;
                    break;
                }
                probe += 1;
            }
            if has_float_field {
                output.push(line.to_owned());
            } else {
                let indent = &line[..line.len() - line.trim_start().len()];
                output.push(format!(
                    "{indent}#[derive(Clone, PartialEq, Eq, ::prost::Message)]"
                ));
            }
        } else {
            output.push(line.to_owned());
        }
        index += 1;
    }
    output.join("\n") + "\n"
}
