package net.tikeo.wasm;

import net.tikeo.processor.TaskOutcome;
import net.tikeo.script.ScriptRunnerLogSink;

/**
 * Executes a released WASM processor snapshot inside a sandbox runtime.
 */
@FunctionalInterface
public interface WasmRunner {
    TaskOutcome run(WasmRunnerTask task, ScriptRunnerLogSink logSink) throws Exception;

    default TaskOutcome run(WasmRunnerTask task) throws Exception {
        return run(task, ScriptRunnerLogSink.NOOP);
    }
}
