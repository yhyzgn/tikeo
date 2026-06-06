#![allow(clippy::redundant_pub_crate)]

use crate::{proto::worker::v1::DispatchTask, task::TaskOutcome};

#[cfg(feature = "wasm")]
pub(crate) fn process_wasm_binding(
    binding: &crate::proto::worker::v1::WasmProcessorBinding,
    _task: &DispatchTask,
) -> TaskOutcome {
    match wasm_runtime::execute(binding) {
        Ok(()) => TaskOutcome::Succeeded,
        Err(error) => TaskOutcome::Failed(error),
    }
}

#[cfg(not(feature = "wasm"))]
pub(crate) fn process_wasm_binding(
    binding: &crate::proto::worker::v1::WasmProcessorBinding,
    _task: &DispatchTask,
) -> TaskOutcome {
    TaskOutcome::Failed(format!(
        "wasm processor binding for script {} requires enabling tikeo feature 'wasm'",
        binding.script_id
    ))
}

#[cfg(feature = "wasm")]
mod wasm_runtime {
    use std::{thread, time::Duration};

    use sha2::{Digest, Sha256};
    use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimitsBuilder};

    use crate::proto::worker::v1::WasmProcessorBinding;

    pub fn execute(binding: &WasmProcessorBinding) -> Result<(), String> {
        validate(binding)?;
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|error| format!("wasm engine error: {error}"))?;
        let module = Module::from_binary(&engine, &binding.module)
            .map_err(|error| format!("wasm module error: {error}"))?;
        let memory_size = usize::try_from(binding.max_memory_bytes).unwrap_or(usize::MAX);
        let limits = StoreLimitsBuilder::new().memory_size(memory_size).build();
        let mut store = Store::new(&engine, limits);
        store
            .set_fuel(binding.fuel)
            .map_err(|error| format!("wasm fuel error: {error}"))?;
        store.limiter(|limits| limits);
        let timeout = Duration::from_millis(binding.timeout_ms);
        let deadline_engine = engine.clone();
        let _interrupter = thread::spawn(move || {
            thread::sleep(timeout);
            deadline_engine.increment_epoch();
        });
        store.set_epoch_deadline(1);
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| format!("wasm instantiate error: {error}"))?;
        let entrypoint = instance
            .get_typed_func::<(), ()>(&mut store, &binding.entrypoint)
            .map_err(|error| format!("wasm entrypoint error: {error}"))?;
        entrypoint
            .call(&mut store, ())
            .map_err(|error| format!("wasm trap: {error}"))
    }

    fn validate(binding: &WasmProcessorBinding) -> Result<(), String> {
        if !binding.module_sha256.trim().is_empty() {
            let actual = format!("{:x}", Sha256::digest(&binding.module));
            if !actual.eq_ignore_ascii_case(binding.module_sha256.trim()) {
                return Err("wasm module sha256 digest mismatch".to_owned());
            }
        }
        if binding.runtime != "wasmtime" {
            return Err(format!("unsupported wasm runtime: {}", binding.runtime));
        }
        if binding.entrypoint.trim().is_empty() {
            return Err("wasm entrypoint must not be empty".to_owned());
        }
        if binding.timeout_ms == 0 {
            return Err("wasm timeout must be greater than zero".to_owned());
        }
        if binding.max_memory_bytes == 0 {
            return Err("wasm memory limit must be greater than zero".to_owned());
        }
        if binding.fuel == 0 {
            return Err("wasm fuel budget must be greater than zero".to_owned());
        }
        if binding.allow_network {
            return Err(
                "wasm network capability is not supported by the Rust SDK adapter yet".to_owned(),
            );
        }
        Ok(())
    }
}
