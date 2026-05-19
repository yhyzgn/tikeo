package cn.recycloud.scheduler.sdk.spring;

import java.lang.reflect.Method;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

import cn.recycloud.scheduler.sdk.core.SchedulerProcessor;
import org.springframework.beans.BeansException;
import org.springframework.beans.factory.config.BeanPostProcessor;
import org.springframework.core.annotation.AnnotationUtils;

/**
 * Discovers {@link SchedulerProcessor} annotations from Spring beans.
 */
public class SchedulerProcessorRegistry implements BeanPostProcessor {
    private final Map<String, Object> processors = new LinkedHashMap<>();

    public Map<String, Object> processors() {
        return Collections.unmodifiableMap(processors);
    }

    @Override
    public Object postProcessAfterInitialization(Object bean, String beanName) throws BeansException {
        var typeAnnotation = AnnotationUtils.findAnnotation(bean.getClass(), SchedulerProcessor.class);
        if (typeAnnotation != null) {
            processors.put(typeAnnotation.value(), bean);
        }
        for (Method method : bean.getClass().getMethods()) {
            var methodAnnotation = AnnotationUtils.findAnnotation(method, SchedulerProcessor.class);
            if (methodAnnotation != null) {
                processors.put(methodAnnotation.value(), bean);
            }
        }
        return bean;
    }
}
