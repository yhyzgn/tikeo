package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;

/** Worker-side sandbox runner for dynamic scripts. */
public interface ScriptRunner {
    ScriptRunnerKind kind();

    /**
     * Whether this runner represents an actually executable sandbox boundary that may be
     * advertised to the server as structured Worker capability.
     */
    default boolean advertiseCapability() {
        return true;
    }

    /**
     * Structured sandbox backend value advertised for this runner.
     */
    default ScriptSandboxBackend advertisedBackend() {
        return ScriptSandboxBackend.AUTO.resolve(kind());
    }

    TaskOutcome run(ScriptRunnerTask task) throws Exception;

    default TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) throws Exception {
        return run(task);
    }
}
