//! Generated tikeo protobuf and gRPC bindings.

#![forbid(unsafe_code)]

/// Worker tunnel protocol bindings.
pub mod worker {
    /// Version 1 worker tunnel protocol.
    pub mod v1 {
        mod generated {
            tonic::include_proto!("tikeo.worker.v1");
        }

        pub use generated::*;
    }
}
