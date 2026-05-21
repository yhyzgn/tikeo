package cn.recycloud.scheduler.sdk.core;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a bean type or method as a scheduler task processor.
 */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
public @interface SchedulerProcessor {
    /**
     * Stable processor name used by scheduler job definitions.
     *
     * @return processor name
     */
    String value();
}
