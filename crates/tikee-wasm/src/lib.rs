//! Worker-side WASM runtime executor.
//!
//! This crate is intentionally separate from server HTTP/storage crates so the heavy Wasmtime
//! dependency stays at the worker execution boundary.

#![forbid(unsafe_code)]

use std::{sync::Arc, thread, time::Duration};

use thiserror::Error;
use tikee_core::{WasmCapabilities, WasmProcessorSpec, WasmSpecError};
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

/// Compiled worker-side WASM executor.
#[derive(Clone)]
pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    /// Build a new executor with fuel metering enabled.
    ///
    /// # Errors
    ///
    /// Returns [`WasmExecutionError`] when the underlying Wasmtime engine cannot be created.
    pub fn new() -> Result<Self, WasmExecutionError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(WasmExecutionError::Engine)?;
        Ok(Self { engine })
    }

    /// Execute a module with the provided tikee WASM policy.
    ///
    /// The current Phase 3 executor supports command-style modules whose exported entrypoint does
    /// not require parameters and does not return values. WASI imports are deliberately not wired
    /// yet, so modules have no ambient filesystem, environment, or network access.
    ///
    /// # Errors
    ///
    /// Returns [`WasmExecutionError`] when validation fails, the module does not compile, the
    /// entrypoint is missing or has the wrong shape, or execution traps/times out.
    pub fn execute(
        &self,
        module_bytes: &[u8],
        spec: &WasmProcessorSpec,
    ) -> Result<(), WasmExecutionError> {
        spec.validate().map_err(WasmExecutionError::Policy)?;
        validate_phase3_capabilities(&spec.capabilities)?;

        let module =
            Module::from_binary(&self.engine, module_bytes).map_err(WasmExecutionError::Module)?;
        let mut store = Store::new(&self.engine, RuntimeState::new(spec));
        store
            .set_fuel(spec.resources.fuel)
            .map_err(WasmExecutionError::Fuel)?;
        store.limiter(|state| &mut state.limits);

        let engine = self.engine.clone();
        let timeout = Duration::from_millis(spec.resources.timeout_ms);
        let _interrupter = thread::spawn(move || {
            thread::sleep(timeout);
            engine.increment_epoch();
        });

        store.set_epoch_deadline(1);
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(WasmExecutionError::Instantiate)?;
        let entrypoint = instance
            .get_typed_func::<(), ()>(&mut store, &spec.entrypoint)
            .map_err(WasmExecutionError::Entrypoint)?;
        entrypoint
            .call(&mut store, ())
            .map_err(WasmExecutionError::Trap)
    }
}

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|error| panic!("create wasm executor: {error}"))
    }
}

struct RuntimeState {
    limits: StoreLimits,
}

impl RuntimeState {
    fn new(spec: &WasmProcessorSpec) -> Self {
        let memory_size = usize::try_from(spec.resources.max_memory_bytes).unwrap_or(usize::MAX);
        Self {
            limits: StoreLimitsBuilder::new().memory_size(memory_size).build(),
        }
    }
}

const fn validate_phase3_capabilities(
    capabilities: &WasmCapabilities,
) -> Result<(), WasmExecutionError> {
    if capabilities.network {
        return Err(WasmExecutionError::Policy(
            WasmSpecError::NetworkNotSupported,
        ));
    }
    if !capabilities.preopened_dirs.is_empty() {
        return Err(WasmExecutionError::Policy(
            WasmSpecError::FilesystemNotSupported,
        ));
    }
    Ok(())
}

/// Errors returned by the worker-side WASM executor.
#[derive(Debug, Error)]
pub enum WasmExecutionError {
    /// Policy validation failed before runtime setup.
    #[error("invalid wasm policy: {0}")]
    Policy(WasmSpecError),
    /// Wasmtime engine creation failed.
    #[error("wasm engine error: {0}")]
    Engine(wasmtime::Error),
    /// Module compilation failed.
    #[error("wasm module error: {0}")]
    Module(wasmtime::Error),
    /// Fuel setup failed.
    #[error("wasm fuel error: {0}")]
    Fuel(wasmtime::Error),
    /// Module instantiation failed.
    #[error("wasm instantiate error: {0}")]
    Instantiate(wasmtime::Error),
    /// Entrypoint lookup/type check failed.
    #[error("wasm entrypoint error: {0}")]
    Entrypoint(wasmtime::Error),
    /// Module execution trapped or was interrupted.
    #[error("wasm trap: {0}")]
    Trap(wasmtime::Error),
}

/// Shared executor handle for worker integrations.
pub type SharedWasmExecutor = Arc<WasmExecutor>;

#[cfg(test)]
mod tests {
    use tikee_core::{WasmCapabilities, WasmProcessorSpec, WasmSpecError};

    use super::{WasmExecutionError, WasmExecutor};

    fn wat(source: &str) -> Vec<u8> {
        match wat::parse_str(source) {
            Ok(bytes) => bytes,
            Err(error) => panic!("compile wat fixture: {error}"),
        }
    }

    #[test]
    fn executes_minimal_entrypoint() {
        let executor = WasmExecutor::new().unwrap_or_else(|error| panic!("executor: {error}"));
        let module = wat(r#"(module (func (export "_start")))"#);

        executor
            .execute(&module, &WasmProcessorSpec::default())
            .unwrap_or_else(|error| panic!("execute wasm: {error}"));
    }

    #[test]
    fn rejects_network_capability_before_runtime() {
        let executor = WasmExecutor::new().unwrap_or_else(|error| panic!("executor: {error}"));
        let module = wat(r#"(module (func (export "_start")))"#);
        let spec = WasmProcessorSpec {
            capabilities: WasmCapabilities {
                network: true,
                ..WasmCapabilities::default()
            },
            ..WasmProcessorSpec::default()
        };

        let error = match executor.execute(&module, &spec) {
            Ok(()) => panic!("network capability should fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            WasmExecutionError::Policy(WasmSpecError::NetworkNotSupported)
        ));
    }

    #[test]
    fn rejects_missing_entrypoint() {
        let executor = WasmExecutor::new().unwrap_or_else(|error| panic!("executor: {error}"));
        let module = wat(r#"(module (func (export "run")))"#);

        let error = match executor.execute(&module, &WasmProcessorSpec::default()) {
            Ok(()) => panic!("missing entrypoint should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, WasmExecutionError::Entrypoint(_)));
    }

    #[test]
    fn fuel_limit_interrupts_busy_loop() {
        let executor = WasmExecutor::new().unwrap_or_else(|error| panic!("executor: {error}"));
        let module = wat(r#"(module
              (func (export "_start")
                (loop br 0)))"#);
        let spec = WasmProcessorSpec {
            resources: tikee_core::WasmResourcePolicy {
                fuel: 10,
                ..tikee_core::WasmResourcePolicy::default()
            },
            ..WasmProcessorSpec::default()
        };

        let error = match executor.execute(&module, &spec) {
            Ok(()) => panic!("busy loop should exhaust fuel"),
            Err(error) => error,
        };

        assert!(matches!(error, WasmExecutionError::Trap(_)));
    }
}
