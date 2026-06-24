package net.tikeo.processor;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a bean type or method as a tikeo task processor.
 */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
public @interface TikeoProcessor {
    /**
     * Stable processor name used by tikeo job definitions.
     *
     * @return processor name
     */
    String value();

    /**
     * Structured processor category. The {@link #value()} remains the executor name.
     *
     * @return processor kind
     */
    TikeoProcessorKind kind() default TikeoProcessorKind.NORMAL;

    /**
     * Optional processor description shown in operator UI.
     *
     * @return human-readable processor description
     */
    String description() default "";

    /**
     * Plugin processor type when {@link #kind()} is {@link TikeoProcessorKind#PLUGIN}.
     *
     * @return constrained plugin processor type
     */
    TikeoPluginType pluginType() default TikeoPluginType.NONE;

    /**
     * Custom plugin processor type when {@link #pluginType()} is {@link TikeoPluginType#CUSTOM}.
     *
     * @return custom plugin processor type
     */
    String customPluginType() default "";
}
