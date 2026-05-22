package com.yhyzgn.tikee.spring;

import com.yhyzgn.tikee.core.TikeeProcessor;
import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.core.TaskOutcome;
import java.lang.reflect.Method;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import org.springframework.beans.BeansException;
import org.springframework.beans.factory.config.BeanPostProcessor;
import org.springframework.core.annotation.AnnotationUtils;

/**
 * Discovers {@link TikeeProcessor} annotations from Spring beans.
 */
public class TikeeProcessorRegistry implements BeanPostProcessor {
    private final Map<String, TikeeProcessorHandler> handlers = new LinkedHashMap<>();

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

    @Override
    public Object postProcessAfterInitialization(Object bean, String beanName) throws BeansException {
        var typeAnnotation = AnnotationUtils.findAnnotation(bean.getClass(), TikeeProcessor.class);
        if (typeAnnotation != null) {
            registerTypeHandler(typeAnnotation.value(), bean);
        }
        for (Method method : bean.getClass().getMethods()) {
            var methodAnnotation = AnnotationUtils.findAnnotation(method, TikeeProcessor.class);
            if (methodAnnotation != null) {
                register(methodAnnotation.value(), TikeeProcessorAdapter.forMethod(bean, method));
            }
        }
        return bean;
    }

    private void registerTypeHandler(String processorName, Object bean) {
        if (bean instanceof TikeeProcessorHandler handler) {
            register(processorName, handler);
            return;
        }
        throw new IllegalArgumentException("type-level @TikeeProcessor beans must implement TikeeProcessorHandler: "
                + bean.getClass().getName());
    }

    private void register(String processorName, TikeeProcessorHandler handler) {
        TikeeProcessorHandler existing = handlers.putIfAbsent(processorName, handler);
        if (existing != null) {
            throw new IllegalArgumentException("duplicate tikee processor name: " + processorName);
        }
    }
}
