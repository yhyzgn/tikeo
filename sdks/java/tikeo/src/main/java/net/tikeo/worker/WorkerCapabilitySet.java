package net.tikeo.worker;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Structured worker capabilities used by dispatch routing and operator UI.
 */
public record WorkerCapabilitySet(
        List<String> tags,
        List<String> sdkProcessors,
        List<ScriptRunner> scriptRunners,
        List<PluginProcessor> pluginProcessors) {
    public WorkerCapabilitySet {
        tags = copyClean(tags);
        sdkProcessors = copyClean(sdkProcessors);
        scriptRunners = List.copyOf(scriptRunners == null ? List.of() : scriptRunners);
        pluginProcessors = List.copyOf(pluginProcessors == null ? List.of() : pluginProcessors);
    }

    public static WorkerCapabilitySet empty() {
        return new WorkerCapabilitySet(List.of(), List.of(), List.of(), List.of());
    }

    public static WorkerCapabilitySet tags(List<String> tags) {
        return new WorkerCapabilitySet(tags, List.of(), List.of(), List.of());
    }

    public WorkerCapabilitySet merge(WorkerCapabilitySet other) {
        if (other == null) {
            return this;
        }
        List<String> mergedTags = concat(tags, other.tags);
        List<String> mergedSdk = concat(sdkProcessors, other.sdkProcessors);
        List<ScriptRunner> mergedScripts = new ArrayList<>(scriptRunners);
        mergedScripts.addAll(other.scriptRunners);
        Map<String, PluginProcessor> plugins = new LinkedHashMap<>();
        for (PluginProcessor plugin : pluginProcessors) {
            plugins.put(plugin.type(), plugin);
        }
        for (PluginProcessor plugin : other.pluginProcessors) {
            plugins.merge(plugin.type(), plugin, PluginProcessor::merge);
        }
        return new WorkerCapabilitySet(mergedTags, mergedSdk, mergedScripts, new ArrayList<>(plugins.values()));
    }

    private static List<String> concat(List<String> left, List<String> right) {
        var values = new ArrayList<String>();
        values.addAll(left);
        values.addAll(right);
        return copyClean(values);
    }

    private static List<String> copyClean(List<String> values) {
        return values == null ? List.of() : values.stream()
                .filter(Objects::nonNull)
                .map(String::trim)
                .filter(value -> !value.isEmpty())
                .distinct()
                .toList();
    }

    /**
 * Structured script runtime declaration.
 */
    public record ScriptRunner(String language, String sandboxBackend) {
        public ScriptRunner {
            language = requireClean(language, "language");
            sandboxBackend = sandboxBackend == null || sandboxBackend.isBlank() ? "auto" : sandboxBackend.trim();
        }
    }

    /**
 * Structured plugin processor declaration.
 */
    public record PluginProcessor(String type, List<String> processorNames) {
        public PluginProcessor {
            type = requireClean(type, "type");
            processorNames = copyClean(processorNames);
        }

        PluginProcessor merge(PluginProcessor other) {
            return new PluginProcessor(type, concat(processorNames, other.processorNames));
        }
    }

    private static String requireClean(String value, String field) {
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException("worker capability " + field + " must not be blank");
        }
        return value.trim();
    }
}
