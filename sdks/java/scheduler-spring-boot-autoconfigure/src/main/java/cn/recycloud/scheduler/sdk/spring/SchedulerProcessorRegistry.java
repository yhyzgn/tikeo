package cn.recycloud.scheduler.sdk.spring;

import cn.recycloud.scheduler.sdk.core.SchedulerProcessor;
import cn.recycloud.scheduler.sdk.core.TaskContext;
import cn.recycloud.scheduler.sdk.core.TaskOutcome;
import java.lang.reflect.Method;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import org.springframework.beans.BeansException;
import org.springframework.beans.factory.config.BeanPostProcessor;
import org.springframework.core.annotation.AnnotationUtils;

/**
 * Discovers {@link SchedulerProcessor} annotations from Spring beans.
 */
public class SchedulerProcessorRegistry implements BeanPostProcessor {
    private final Map<String, SchedulerProcessorHandler> handlers = new LinkedHashMap<>();

    /**
     * Registered processor handlers keyed by processor name.
     *
     * @return immutable processor handler map
     */
    public Map<String, SchedulerProcessorHandler> handlers() {
        return Collections.unmodifiableMap(handlers);
    }

    /**
     * Backward-compatible view of registered processor names.
     *
     * @return immutable processor map
     */
    public Map<String, SchedulerProcessorHandler> processors() {
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
        SchedulerProcessorHandler handler = handlers.get(processorName);
        if (handler == null) {
            return TaskOutcome.failed("no scheduler processor registered: " + processorName);
        }
        return handler.invoke(context);
    }

    @Override
    public Object postProcessAfterInitialization(Object bean, String beanName) throws BeansException {
        var typeAnnotation = AnnotationUtils.findAnnotation(bean.getClass(), SchedulerProcessor.class);
        if (typeAnnotation != null) {
            registerTypeHandler(typeAnnotation.value(), bean);
        }
        for (Method method : bean.getClass().getMethods()) {
            var methodAnnotation = AnnotationUtils.findAnnotation(method, SchedulerProcessor.class);
            if (methodAnnotation != null) {
                register(methodAnnotation.value(), SchedulerProcessorAdapter.forMethod(bean, method));
            }
        }
        return bean;
    }

    private void registerTypeHandler(String processorName, Object bean) {
        if (bean instanceof SchedulerProcessorHandler handler) {
            register(processorName, handler);
            return;
        }
        throw new IllegalArgumentException("type-level @SchedulerProcessor beans must implement SchedulerProcessorHandler: "
                + bean.getClass().getName());
    }

    private void register(String processorName, SchedulerProcessorHandler handler) {
        SchedulerProcessorHandler existing = handlers.putIfAbsent(processorName, handler);
        if (existing != null) {
            throw new IllegalArgumentException("duplicate scheduler processor name: " + processorName);
        }
    }
}
