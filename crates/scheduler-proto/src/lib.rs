//! Generated scheduler protobuf and gRPC bindings.

#![forbid(unsafe_code)]

/// Worker tunnel protocol bindings.
pub mod worker {
    /// Version 1 worker tunnel protocol.
    pub mod v1 {
        #![allow(
            missing_docs,
            clippy::default_trait_access,
            clippy::derive_partial_eq_without_eq,
            clippy::doc_markdown,
            clippy::missing_const_for_fn
        )]
        tonic::include_proto!("scheduler.worker.v1");
    }
}
