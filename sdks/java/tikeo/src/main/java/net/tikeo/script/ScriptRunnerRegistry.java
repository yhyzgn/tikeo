package net.tikeo.script;

import net.tikeo.worker.WorkerCapabilitySet;
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
        List<String> advertised = runners.values().stream()
                .filter(ScriptRunner::advertiseCapability)
                .map(runner -> runner.kind().capability())
                .distinct()
                .sorted()
                .toList();
        if (advertised.isEmpty()) {
            return List.of();
        }
        return java.util.stream.Stream.concat(java.util.stream.Stream.of("script"), advertised.stream())
                .distinct()
                .sorted()
                .toList();
    }

    public List<WorkerCapabilitySet.ScriptRunner> structuredCapabilities() {
        return runners.values().stream()
                .filter(ScriptRunner::advertiseCapability)
                .map(runner -> new WorkerCapabilitySet.ScriptRunner(
                        runner.kind().value(),
                        runner.advertisedBackend().value()))
                .sorted(java.util.Comparator.comparing(WorkerCapabilitySet.ScriptRunner::language))
                .toList();
    }

    public boolean isEmpty() {
        return runners.isEmpty();
    }
}
