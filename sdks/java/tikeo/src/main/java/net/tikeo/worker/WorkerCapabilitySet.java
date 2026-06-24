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
        List<Processor> normalProcessors,
        List<ScriptRunner> scriptRunners,
        List<PluginProcessor> pluginProcessors) {
    public WorkerCapabilitySet {
        tags = copyClean(tags);
        normalProcessors = copyProcessors(normalProcessors);
        scriptRunners = List.copyOf(scriptRunners == null ? List.of() : scriptRunners);
        pluginProcessors = List.copyOf(pluginProcessors == null ? List.of() : pluginProcessors);
    }

    public WorkerCapabilitySet(
            List<String> tags,
            List<String> normalProcessorNames,
            List<ScriptRunner> scriptRunners,
            List<PluginProcessor> pluginProcessors,
            boolean fromNames) {
        this(tags, processorsFromNames(normalProcessorNames), scriptRunners, pluginProcessors);
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
        List<Processor> mergedNormal = concatProcessors(normalProcessors, other.normalProcessors);
        List<ScriptRunner> mergedScripts = new ArrayList<>(scriptRunners);
        mergedScripts.addAll(other.scriptRunners);
        Map<String, PluginProcessor> plugins = new LinkedHashMap<>();
        for (PluginProcessor plugin : pluginProcessors) {
            plugins.put(plugin.type(), plugin);
        }
        for (PluginProcessor plugin : other.pluginProcessors) {
            plugins.merge(plugin.type(), plugin, PluginProcessor::merge);
        }
        return new WorkerCapabilitySet(mergedTags, mergedNormal, mergedScripts, new ArrayList<>(plugins.values()));
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

    private static List<Processor> processorsFromNames(List<String> names) {
        return copyClean(names).stream().map(name -> new Processor(name, "")).toList();
    }

    private static List<Processor> copyProcessors(List<Processor> values) {
        if (values == null) {
            return List.of();
        }
        Map<String, Processor> byName = new LinkedHashMap<>();
        for (Processor processor : values) {
            if (processor == null) {
                continue;
            }
            byName.putIfAbsent(processor.name(), processor);
        }
        return List.copyOf(byName.values());
    }

    private static List<Processor> concatProcessors(List<Processor> left, List<Processor> right) {
        Map<String, Processor> byName = new LinkedHashMap<>();
        for (Processor processor : left) {
            byName.put(processor.name(), processor);
        }
        for (Processor processor : right) {
            byName.merge(processor.name(), processor, Processor::merge);
        }
        return List.copyOf(byName.values());
    }

    /**
     * Structured normal processor declaration.
     */
    public record Processor(String name, String description) {
        public Processor {
            name = requireClean(name, "processor name");
            description = description == null ? "" : description.trim();
        }

        Processor merge(Processor other) {
            return description.isBlank() && !other.description.isBlank() ? other : this;
        }
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
    public record PluginProcessor(String type, List<Processor> processors) {
        public PluginProcessor {
            type = requireClean(type, "type");
            processors = copyProcessors(processors);
        }

        public static PluginProcessor ofNames(String type, List<String> processorNames) {
            return new PluginProcessor(type, processorsFromNames(processorNames));
        }

        public List<String> processorNames() {
            return processors.stream().map(Processor::name).toList();
        }

        PluginProcessor merge(PluginProcessor other) {
            return new PluginProcessor(type, concatProcessors(processors, other.processors));
        }
    }

    private static String requireClean(String value, String field) {
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException("worker capability " + field + " must not be blank");
        }
        return value.trim();
    }
}
