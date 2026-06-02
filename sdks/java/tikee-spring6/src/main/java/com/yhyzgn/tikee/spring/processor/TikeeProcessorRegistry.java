package com.yhyzgn.tikee.spring.processor;

import com.yhyzgn.tikee.processor.TikeeProcessor;
import com.yhyzgn.tikee.processor.TikeeProcessorKind;
import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.worker.WorkerCapabilityProvider;
import com.yhyzgn.tikee.worker.WorkerCapabilitySet;
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
 * Discovers {@link TikeeProcessor} annotations from Spring beans.
 */
public class TikeeProcessorRegistry implements BeanPostProcessor, WorkerCapabilityProvider {
    private final Map<String, TikeeProcessorHandler> handlers = new LinkedHashMap<>();
    private final Map<String, ProcessorDeclaration> declarations = new LinkedHashMap<>();
    private final Set<String> processedBeans = new HashSet<>();

    /**
     * Registered processor handlers keyed by processor name.
     *
     * @return immutable processor handler map
     */
    public Map<String, TikeeProcessorHandler> handlers() {
        return Collections.unmodifiableMap(handlers);
    }

    /**
     * Backward-compatible view of registered processor names.
     *
     * @return immutable processor map
     */
    public Map<String, TikeeProcessorHandler> processors() {
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
                .filter(declaration -> declaration.kind() == TikeeProcessorKind.SDK)
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
                .filter(declaration -> declaration.kind() == TikeeProcessorKind.PLUGIN)
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
        TikeeProcessorHandler handler = handlers.get(processorName);
        if (handler == null) {
            return TaskOutcome.failed("no tikee processor registered: " + processorName);
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
        var typeAnnotation = AnnotationUtils.findAnnotation(bean.getClass(), TikeeProcessor.class);
        if (typeAnnotation != null) {
            registerTypeHandler(typeAnnotation, bean);
        }
        for (Method method : bean.getClass().getMethods()) {
            var methodAnnotation = AnnotationUtils.findAnnotation(method, TikeeProcessor.class);
            if (methodAnnotation != null) {
                register(methodAnnotation, TikeeProcessorAdapter.forMethod(bean, method));
            }
        }
        return bean;
    }

    private void registerTypeHandler(TikeeProcessor annotation, Object bean) {
        if (bean instanceof TikeeProcessorHandler handler) {
            register(annotation, handler);
            return;
        }
        throw new IllegalArgumentException("type-level @TikeeProcessor beans must implement TikeeProcessorHandler: "
                + bean.getClass().getName());
    }

    private void register(TikeeProcessor annotation, TikeeProcessorHandler handler) {
        String processorName = annotation.value();
        if (processorName == null || processorName.isBlank()) {
            throw new IllegalArgumentException("tikee processor name must not be blank");
        }
        if (processorName.startsWith("script:")) {
            throw new IllegalArgumentException("@TikeeProcessor is reserved for SDK processors; use script runner capabilities for script executors");
        }
        String pluginType = annotation.pluginType() == null ? "" : annotation.pluginType().trim();
        if (annotation.kind() == TikeeProcessorKind.PLUGIN && pluginType.isBlank()) {
            throw new IllegalArgumentException("plugin @TikeeProcessor requires non-blank pluginType: " + processorName);
        }
        if (annotation.kind() == TikeeProcessorKind.SDK && !pluginType.isBlank()) {
            throw new IllegalArgumentException("SDK @TikeeProcessor must not declare pluginType: " + processorName);
        }
        TikeeProcessorHandler existing = handlers.putIfAbsent(processorName, handler);
        if (existing != null) {
            throw new IllegalArgumentException("duplicate tikee processor name: " + processorName);
        }
        declarations.put(
                processorName,
                new ProcessorDeclaration(processorName.trim(), annotation.kind(), pluginType));
    }

    private record ProcessorDeclaration(String name, TikeeProcessorKind kind, String pluginType) {}
}
