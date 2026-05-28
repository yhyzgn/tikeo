package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.worker.WorkerCapabilitySet;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

/** Explicit registry of sandboxed script runners enabled by a worker. */
public final class ScriptRunnerRegistry {
    private final Map<ScriptRunnerKind, ScriptRunner> runners = new EnumMap<>(ScriptRunnerKind.class);

    public ScriptRunnerRegistry register(ScriptRunner runner) {
        runners.put(runner.kind(), runner);
        return this;
    }

    public Optional<ScriptRunner> find(ScriptRunnerKind kind) {
        return Optional.ofNullable(runners.get(kind));
    }

    public List<String> capabilities() {
        if (runners.isEmpty()) {
            return List.of();
        }
        return java.util.stream.Stream.concat(
                        java.util.stream.Stream.of("script"),
                        runners.keySet().stream().map(ScriptRunnerKind::capability))
                .distinct()
                .sorted()
                .toList();
    }

    public List<WorkerCapabilitySet.ScriptRunner> structuredCapabilities() {
        return runners.keySet().stream()
                .map(kind -> new WorkerCapabilitySet.ScriptRunner(kind.value(), "auto"))
                .toList();
    }

    public boolean isEmpty() {
        return runners.isEmpty();
    }
}
