package net.tikeo.spring.processor;

import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.worker.WorkerCapabilityProvider;
import net.tikeo.worker.WorkerCapabilitySet;
import java.lang.reflect.Method;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
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
     * Backward-compatible view of registered processor names.
     *
     * @return immutable processor map
     */
    public Map<String, TikeoProcessorHandler> processors() {
        return handlers();
    }

    /**
     * Legacy registered SDK processor capabilities for Worker registration.
     *
     * @return immutable capability list using the processor:&lt;name&gt; convention
     */
    public List<String> processorCapabilities() {
        return sdkProcessorNames().stream()
                .map(name -> "processor:" + name)
                .toList();
    }

    /**
     * Registered normal SDK processor names.
     *
     * @return immutable SDK processor name list
     */
    public List<String> sdkProcessorNames() {
        return declarations.values().stream()
                .filter(declaration -> declaration.kind() == TikeoProcessorKind.SDK)
                .map(ProcessorDeclaration::name)
                .toList();
    }

    /**
     * Registered plugin processors grouped by explicit plugin type.
     *
     * @return immutable plugin processor declaration list
     */
    public List<WorkerCapabilitySet.PluginProcessor> pluginProcessors() {
        Map<String, List<String>> byType = new LinkedHashMap<>();
        declarations.values().stream()
                .filter(declaration -> declaration.kind() == TikeoProcessorKind.PLUGIN)
                .forEach(declaration -> byType
                        .computeIfAbsent(declaration.pluginType(), ignored -> new java.util.ArrayList<>())
                        .add(declaration.name()));
        return byType.entrySet().stream()
                .map(entry -> new WorkerCapabilitySet.PluginProcessor(entry.getKey(), entry.getValue()))
                .toList();
    }

    @Override
    public WorkerCapabilitySet workerCapabilities() {
        return new WorkerCapabilitySet(List.of(), sdkProcessorNames(), List.of(), pluginProcessors());
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
            throw new IllegalArgumentException("@TikeoProcessor is reserved for SDK processors; use script runner capabilities for script executors");
        }
        String pluginType = annotation.pluginType() == null ? "" : annotation.pluginType().trim();
        if (annotation.kind() == TikeoProcessorKind.PLUGIN && pluginType.isBlank()) {
            throw new IllegalArgumentException("plugin @TikeoProcessor requires non-blank pluginType: " + processorName);
        }
        if (annotation.kind() == TikeoProcessorKind.SDK && !pluginType.isBlank()) {
            throw new IllegalArgumentException("SDK @TikeoProcessor must not declare pluginType: " + processorName);
        }
        TikeoProcessorHandler existing = handlers.putIfAbsent(processorName, handler);
        if (existing != null) {
            throw new IllegalArgumentException("duplicate tikeo processor name: " + processorName);
        }
        declarations.put(
                processorName,
                new ProcessorDeclaration(processorName.trim(), annotation.kind(), pluginType));
    }

    private record ProcessorDeclaration(String name, TikeoProcessorKind kind, String pluginType) {}
}
