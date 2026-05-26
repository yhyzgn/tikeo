package com.yhyzgn.tikee.wasm;

import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.script.ScriptRunnerLogSink;

/** Executes a released WASM processor snapshot inside a sandbox runtime. */
@FunctionalInterface
public interface WasmRunner {
    TaskOutcome run(WasmRunnerTask task, ScriptRunnerLogSink logSink) throws Exception;

    default TaskOutcome run(WasmRunnerTask task) throws Exception {
        return run(task, ScriptRunnerLogSink.NOOP);
    }
}
