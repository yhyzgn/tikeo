package com.yhyzgn.tikee.spring;

import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.core.TaskOutcome;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Objects;

/**
 * Adapts {@code @TikeeProcessor} methods to the core {@link com.yhyzgn.tikee.core.TaskProcessor} model.
 */
final class TikeeProcessorAdapter implements TikeeProcessorHandler {
    private final Object bean;
    private final Method method;

    private TikeeProcessorAdapter(Object bean, Method method) {
        this.bean = Objects.requireNonNull(bean, "bean");
        this.method = Objects.requireNonNull(method, "method");
        this.method.setAccessible(true);
        validate(method);
    }

    static TikeeProcessorHandler forMethod(Object bean, Method method) {
        return new TikeeProcessorAdapter(bean, method);
    }

    @Override
    public TaskOutcome invoke(TaskContext context) {
        Objects.requireNonNull(context, "context");
        try {
            Object result = method.invoke(bean, arguments(context));
            return toOutcome(result);
        } catch (InvocationTargetException error) {
            Throwable target = error.getTargetException();
            return TaskOutcome.failed(target == null ? error.getMessage() : target.getMessage());
        } catch (ReflectiveOperationException | IllegalArgumentException error) {
            return TaskOutcome.failed(error.getMessage());
        }
    }

    private Object[] arguments(TaskContext context) {
        Class<?>[] parameterTypes = method.getParameterTypes();
        if (parameterTypes.length == 0) {
            return new Object[0];
        }
        Class<?> parameterType = parameterTypes[0];
        if (TaskContext.class.equals(parameterType)) {
            return new Object[] {context};
        }
        if (String.class.equals(parameterType)) {
            return new Object[] {new String(context.payload(), StandardCharsets.UTF_8)};
        }
        if (byte[].class.equals(parameterType)) {
            return new Object[] {context.payload()};
        }
        throw new IllegalArgumentException("unsupported processor parameter type: " + parameterType.getName());
    }

    private static TaskOutcome toOutcome(Object result) {
        if (result == null) {
            return TaskOutcome.succeeded();
        }
        if (result instanceof TaskOutcome outcome) {
            return outcome;
        }
        if (result instanceof String message) {
            return new TaskOutcome(true, message);
        }
        if (result instanceof Boolean success) {
            return new TaskOutcome(success, "");
        }
        return TaskOutcome.failed("unsupported processor return type: " + result.getClass().getName());
    }

    private static void validate(Method method) {
        if (method.getParameterCount() > 1) {
            throw new IllegalArgumentException("@TikeeProcessor method must have zero or one parameter: " + method);
        }
        if (method.getParameterCount() == 1) {
            Class<?> parameterType = method.getParameterTypes()[0];
            boolean supported = Arrays.asList(TaskContext.class, String.class, byte[].class).contains(parameterType);
            if (!supported) {
                throw new IllegalArgumentException("unsupported @TikeeProcessor parameter type: " + parameterType.getName());
            }
        }
        Class<?> returnType = method.getReturnType();
        boolean supportedReturn = Void.TYPE.equals(returnType)
                || TaskOutcome.class.equals(returnType)
                || String.class.equals(returnType)
                || Boolean.TYPE.equals(returnType)
                || Boolean.class.equals(returnType);
        if (!supportedReturn) {
            throw new IllegalArgumentException("unsupported @TikeeProcessor return type: " + returnType.getName());
        }
    }
}
