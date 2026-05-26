package com.yhyzgn.tikee.script;

/** Immutable script snapshot passed to a sandbox runner. */
public record ScriptRunnerTask(
        String scriptId,
        String versionId,
        long versionNumber,
        String language,
        String content,
        String contentSha256,
        ScriptRunnerPolicy policy) {}
