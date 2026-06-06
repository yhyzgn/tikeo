package net.tikeo.script;

import net.tikeo.processor.TaskOutcome;

/** Fail-closed runner used to terminate unsupported script languages without queue starvation. */
public final class UnavailableScriptRunner implements ScriptRunner {
    private final ScriptRunnerKind kind;
    private final String reason;

    public UnavailableScriptRunner(ScriptRunnerKind kind, String reason) {
        this.kind = kind;
        this.reason = reason == null || reason.isBlank()
            ? "script runner is unavailable"
            : reason;
    }

    @Override
    public ScriptRunnerKind kind() {
        return kind;
    }

    @Override
    public boolean advertiseCapability() {
        return false;
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task) {
        return run(task, ScriptRunnerLogSink.NOOP);
    }

    @Override
    public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
        ScriptRunnerLogSink sink = logSink == null ? ScriptRunnerLogSink.NOOP : logSink;
        String message = "script runner unavailable for language " + kind.value() + ": " + reason;
        sink.log("error", "[script] " + message);
        return TaskOutcome.failed(message);
    }
}
