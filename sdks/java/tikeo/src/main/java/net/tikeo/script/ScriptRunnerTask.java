package net.tikeo.script;

/** Immutable script snapshot passed to a sandbox runner. */
public record ScriptRunnerTask(
        String scriptId,
        String versionId,
        long versionNumber,
        String language,
        String content,
        String contentSha256,
        ScriptRunnerPolicy policy,
        ScriptSandboxBackend sandboxBackend) {
    public ScriptRunnerTask {
        sandboxBackend = sandboxBackend == null ? ScriptSandboxBackend.AUTO : sandboxBackend;
    }

    public ScriptRunnerTask(
            String scriptId,
            String versionId,
            long versionNumber,
            String language,
            String content,
            String contentSha256,
            ScriptRunnerPolicy policy) {
        this(scriptId, versionId, versionNumber, language, content, contentSha256, policy, ScriptSandboxBackend.AUTO);
    }
}
