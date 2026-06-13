package net.tikeo.spring.processor;

import java.io.PrintWriter;
import java.io.StringWriter;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.Objects;

/**
 * Adapts {@code @TikeoProcessor} methods to the core {@link net.tikeo.processor.TaskProcessor} model.
 */
final class TikeoProcessorAdapter implements TikeoProcessorHandler {
    private final Object bean;
    private final Method method;

    private TikeoProcessorAdapter(Object bean, Method method) {
        this.bean = Objects.requireNonNull(bean, "bean");
        this.method = Objects.requireNonNull(method, "method");
        this.method.setAccessible(true);
        validate(method);
    }

    static TikeoProcessorHandler forMethod(Object bean, Method method) {
        return new TikeoProcessorAdapter(bean, method);
    }

    @Override
    public TaskOutcome invoke(TaskContext context) {
        Objects.requireNonNull(context, "context");
        try {
            Object result = method.invoke(bean, arguments(context));
            return toOutcome(result);
        } catch (InvocationTargetException error) {
            Throwable target = error.getTargetException() == null ? error : error.getTargetException();
            context.logError(stackTrace(target));
            return TaskOutcome.failed(errorMessage(target));
        } catch (ReflectiveOperationException | IllegalArgumentException error) {
            context.logError(stackTrace(error));
            return TaskOutcome.failed(errorMessage(error));
        }
    }

    private static String errorMessage(Throwable error) {
        if (error.getMessage() == null || error.getMessage().isBlank()) {
            return error.getClass().getName();
        }
        return error.getMessage();
    }

    private static String stackTrace(Throwable error) {
        StringWriter writer = new StringWriter();
        error.printStackTrace(new PrintWriter(writer));
        return writer.toString();
    }

    private Object[] arguments(TaskContext context) {
        Class<?>[] parameterTypes = method.getParameterTypes();
        Object[] arguments = new Object[parameterTypes.length];
        for (int index = 0; index < parameterTypes.length; index++) {
            Class<?> parameterType = parameterTypes[index];
            if (TaskContext.class.equals(parameterType)) {
                arguments[index] = context;
            } else if (String.class.equals(parameterType)) {
                arguments[index] = new String(context.payload(), StandardCharsets.UTF_8);
            } else if (byte[].class.equals(parameterType)) {
                arguments[index] = context.payload();
            } else {
                throw new IllegalArgumentException("unsupported processor parameter type: " + parameterType.getName());
            }
        }
        return arguments;
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
        if (method.getParameterCount() > 2) {
            throw new IllegalArgumentException("@TikeoProcessor method may declare at most TaskContext plus one payload parameter: " + method);
        }
        int contextCount = 0;
        int payloadCount = 0;
        for (Class<?> parameterType : method.getParameterTypes()) {
            if (TaskContext.class.equals(parameterType)) {
                contextCount++;
            } else if (String.class.equals(parameterType) || byte[].class.equals(parameterType)) {
                payloadCount++;
            } else {
                throw new IllegalArgumentException("unsupported @TikeoProcessor parameter type: " + parameterType.getName());
            }
        }
        if (contextCount > 1) {
            throw new IllegalArgumentException("@TikeoProcessor method may declare TaskContext at most once: " + method);
        }
        if (payloadCount > 1) {
            throw new IllegalArgumentException("@TikeoProcessor method may declare at most one payload parameter: " + method);
        }
        Class<?> returnType = method.getReturnType();
        boolean supportedReturn = Void.TYPE.equals(returnType)
                || TaskOutcome.class.equals(returnType)
                || String.class.equals(returnType)
                || Boolean.TYPE.equals(returnType)
                || Boolean.class.equals(returnType);
        if (!supportedReturn) {
            throw new IllegalArgumentException("unsupported @TikeoProcessor return type: " + returnType.getName());
        }
    }
}
