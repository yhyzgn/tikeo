package net.tikeo.script;

/**
 * Receives stdout/stderr emitted by a sandboxed script runner.
 */
@FunctionalInterface
public interface ScriptRunnerLogSink {
    ScriptRunnerLogSink NOOP = (level, message) -> {};

    void log(String level, String message);
}
