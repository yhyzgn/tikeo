package net.tikeo.wasm;

/** Exception raised when a WASM sandbox runner rejects or fails a task. */
public class WasmRunnerException extends RuntimeException {
    public WasmRunnerException(String message) {
        super(message);
    }

    public WasmRunnerException(String message, Throwable cause) {
        super(message, cause);
    }
}
