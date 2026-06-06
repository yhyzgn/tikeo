//! Generated Worker Tunnel protocol bindings bundled for standalone SDK publishing.

/// Worker tunnel protocol bindings.
pub mod worker {
    /// Version 1 worker tunnel protocol.
    pub mod v1 {
        #![allow(
            missing_docs,
            clippy::default_trait_access,
            clippy::derive_partial_eq_without_eq,
            clippy::doc_markdown,
            clippy::missing_const_for_fn,
            clippy::missing_errors_doc,
            clippy::too_many_lines
        )]
        tonic::include_proto!("tikeo.worker.v1");
    }
}
