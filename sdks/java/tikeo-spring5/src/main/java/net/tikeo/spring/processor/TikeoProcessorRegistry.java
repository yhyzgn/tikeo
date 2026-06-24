package net.tikeo.spring.processor;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.processor.TikeoPluginType;
import net.tikeo.worker.WorkerCapabilityProvider;
import net.tikeo.worker.WorkerCapabilitySet;
import org.springframework.beans.BeansException;
import org.springframework.beans.factory.config.BeanPostProcessor;
import org.springframework.context.ApplicationContext;
import org.springframework.core.annotation.AnnotationUtils;

/**
 * Discovers {@link TikeoProcessor} annotations from Spring beans.
 */
public class TikeoProcessorRegistry implements BeanPostProcessor, WorkerCapabilityProvider {
    private final Map<String, TikeoProcessorHandler> handlers = new LinkedHashMap<>();
    private final Map<String, ProcessorDeclaration> declarations = new LinkedHashMap<>();
    private final Set<String> processedBeans = new HashSet<>();

    /**
     * Registered processor handlers keyed by processor name.
     *
     * @return immutable processor handler map
     */
    public Map<String, TikeoProcessorHandler> handlers() {
        return Collections.unmodifiableMap(handlers);
    }

    /**
     * View of registered processor names.
     *
     * @return immutable processor map
     */
    public Map<String, TikeoProcessorHandler> processors() {
        return handlers();
    }

    /**
     * Registered normal processor names.
     *
     * @return immutable normal processor name list
     */
    public List<String> normalProcessorNames() {
        return declarations.values().stream()
                .filter(declaration -> declaration.kind().isNormal())
                .map(ProcessorDeclaration::name)
                .toList();
    }

    /**
     * Registered normal processor declarations.
     *
     * @return immutable processor declaration list
     */
    public List<WorkerCapabilitySet.Processor> normalProcessors() {
        return declarations.values().stream()
                .filter(declaration -> declaration.kind().isNormal())
                .map(declaration -> new WorkerCapabilitySet.Processor(declaration.name(), declaration.description()))
                .toList();
    }

    /**
     * Registered plugin processors grouped by explicit plugin type.
     *
     * @return immutable plugin processor declaration list
     */
    public List<WorkerCapabilitySet.PluginProcessor> pluginProcessors() {
        Map<String, List<WorkerCapabilitySet.Processor>> byType = new LinkedHashMap<>();
        declarations.values().stream()
                .filter(declaration -> declaration.kind() == TikeoProcessorKind.PLUGIN)
                .forEach(declaration -> byType
                        .computeIfAbsent(declaration.pluginType(), ignored -> new ArrayList<>())
                        .add(new WorkerCapabilitySet.Processor(declaration.name(), declaration.description())));
        return byType.entrySet().stream()
                .map(entry -> new WorkerCapabilitySet.PluginProcessor(entry.getKey(), entry.getValue()))
                .toList();
    }

    @Override
    public WorkerCapabilitySet workerCapabilities() {
        return new WorkerCapabilitySet(List.of(), normalProcessors(), List.of(), pluginProcessors());
    }

    /**
     * Invoke a named processor.
     *
     * @param processorName processor name
     * @param context task context
     * @return task outcome
     */
    public TaskOutcome invoke(String processorName, TaskContext context) {
        TikeoProcessorHandler handler = handlers.get(processorName);
        if (handler == null) {
            return TaskOutcome.failed("no tikeo processor registered: " + processorName);
        }
        return handler.invoke(context);
    }

    /**
     * Eagerly discover already-instantiated beans before worker registration is built.
     *
     * @param context Spring application context
     */
    public void scanExistingBeans(ApplicationContext context) {
        for (String beanName : context.getBeanDefinitionNames()) {
            Object bean;
            try {
                bean = context.getBean(beanName);
            } catch (BeansException ignored) {
                continue;
            }
            postProcessAfterInitialization(bean, beanName);
        }
    }

    @Override
    public Object postProcessAfterInitialization(Object bean, String beanName) throws BeansException {
        if (!processedBeans.add(beanName)) {
            return bean;
        }
        var typeAnnotation = AnnotationUtils.findAnnotation(bean.getClass(), TikeoProcessor.class);
        if (typeAnnotation != null) {
            registerTypeHandler(typeAnnotation, bean);
        }
        for (Method method : bean.getClass().getMethods()) {
            var methodAnnotation = AnnotationUtils.findAnnotation(method, TikeoProcessor.class);
            if (methodAnnotation != null) {
                register(methodAnnotation, TikeoProcessorAdapter.forMethod(bean, method));
            }
        }
        return bean;
    }

    private void registerTypeHandler(TikeoProcessor annotation, Object bean) {
        if (bean instanceof TikeoProcessorHandler handler) {
            register(annotation, handler);
            return;
        }
        throw new IllegalArgumentException("type-level @TikeoProcessor beans must implement TikeoProcessorHandler: "
                + bean.getClass().getName());
    }

    private void register(TikeoProcessor annotation, TikeoProcessorHandler handler) {
        String processorName = annotation.value();
        if (processorName == null || processorName.isBlank()) {
            throw new IllegalArgumentException("tikeo processor name must not be blank");
        }
        if (processorName.startsWith("script:")) {
            throw new IllegalArgumentException("@TikeoProcessor is reserved for normal processors; use script runner capabilities for script executors");
        }
        String pluginType = resolvePluginType(annotation);
        if (annotation.kind() == TikeoProcessorKind.PLUGIN && pluginType.isBlank()) {
            throw new IllegalArgumentException("plugin @TikeoProcessor requires non-NONE pluginType: " + processorName);
        }
        if (annotation.kind().isNormal() && !pluginType.isBlank()) {
            throw new IllegalArgumentException("Normal @TikeoProcessor must not declare pluginType: " + processorName);
        }
        TikeoProcessorHandler existing = handlers.putIfAbsent(processorName, handler);
        if (existing != null) {
            throw new IllegalArgumentException("duplicate tikeo processor name: " + processorName);
        }
        declarations.put(
                processorName,
                new ProcessorDeclaration(processorName.trim(), annotation.kind(), pluginType, annotation.description()));
    }

    private static String resolvePluginType(TikeoProcessor annotation) {
        TikeoPluginType pluginType = annotation.pluginType() == null ? TikeoPluginType.NONE : annotation.pluginType();
        if (pluginType == TikeoPluginType.CUSTOM) {
            return annotation.customPluginType() == null ? "" : annotation.customPluginType().trim();
        }
        return pluginType.wireValue();
    }

    private record ProcessorDeclaration(String name, TikeoProcessorKind kind, String pluginType, String description) {
        private ProcessorDeclaration {
            description = description == null ? "" : description.trim();
        }
    }
}
