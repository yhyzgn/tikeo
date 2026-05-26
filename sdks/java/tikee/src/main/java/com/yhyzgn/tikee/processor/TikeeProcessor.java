package com.yhyzgn.tikee.processor;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a bean type or method as a tikee task processor.
 */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
public @interface TikeeProcessor {
    /**
     * Stable processor name used by tikee job definitions.
     *
     * @return processor name
     */
    String value();
}
