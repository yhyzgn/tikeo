package net.tikeo.spring.processor;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.spring.worker.SpringTikeoTaskProcessor;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import org.junit.jupiter.api.Test;

class TikeoProcessorRegistryTest {
    @Test
    void invokesTaskContextMethod() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new ContextBean(), "contextBean");

        TaskOutcome outcome = registry.invoke("demo.context", context("demo.context", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("instance-1:hello");
    }

    @Test
    void invokesStringPayloadMethod() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        TaskOutcome outcome = registry.invoke("demo.string", context("demo.string", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    @Test
    void mapsExceptionsToFailedOutcome() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");

        TaskOutcome outcome = registry.invoke("demo.fail", context("demo.fail", "hello"));

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("boom");
    }

    @Test
    void processorExceptionsEmitStackTraceToTaskLogs() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");
        java.util.List<String> logs = new ArrayList<>();

        TaskOutcome outcome = registry.invoke("demo.fail", new TaskContext("demo.fail", "demo.fail", "instance-1", "hello".getBytes(StandardCharsets.UTF_8), (level, message) -> logs.add(level + ":" + message)));

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("boom");
        assertThat(logs).anySatisfy(line -> {
            assertThat(line).startsWith("error:java.lang.IllegalStateException: boom");
            assertThat(line).contains("FailingBean.run");
        });
    }

    @Test
    void rejectsDuplicateProcessorNames() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        assertThatThrownBy(() -> registry.postProcessAfterInitialization(new DuplicateStringBean(), "duplicate"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("duplicate tikeo processor name");
    }

    @Test
    void exposesOnlySdkProcessorCapabilities() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        registry.postProcessAfterInitialization(new PluginBean(), "pluginBean");

        assertThat(registry.processorCapabilities()).containsExactly("processor:demo.string");
        assertThat(registry.workerCapabilities().sdkProcessors()).containsExactly("demo.string");
        assertThat(registry.workerCapabilities().pluginProcessors())
                .anySatisfy(plugin -> {
                    assertThat(plugin.type()).isEqualTo("sql");
                    assertThat(plugin.processorNames()).containsExactly("billing.sql-sync");
                });
    }

    @Test
    void rejectsPluginProcessorWithoutPluginType() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();

        assertThatThrownBy(() -> registry.postProcessAfterInitialization(new MissingPluginTypeBean(), "pluginBean"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("requires non-blank pluginType");
    }

    @Test
    void rejectsScriptPrefixedProcessorAnnotations() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();

        assertThatThrownBy(() -> registry.postProcessAfterInitialization(new ScriptPrefixedBean(), "scriptBean"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("@TikeoProcessor is reserved for SDK processors");
    }

    @Test
    void routesThroughSpringTaskProcessorByJobId() throws Exception {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        SpringTikeoTaskProcessor processor = new SpringTikeoTaskProcessor(registry);

        TaskOutcome outcome = processor.process(new TaskContext("job-1", "demo.string", "instance-1", "payload".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome).isEqualTo(new TaskOutcome(true, "echo:payload"));
    }

    private static TaskContext context(String jobId, String payload) {
        return new TaskContext(jobId, "instance-1", payload.getBytes(StandardCharsets.UTF_8));
    }

    static final class ContextBean {
        @TikeoProcessor("demo.context")
        public TaskOutcome run(TaskContext context) {
            return new TaskOutcome(true, context.instanceId() + ":" + new String(context.payload(), StandardCharsets.UTF_8));
        }
    }

    static class StringBean {
        @TikeoProcessor("demo.string")
        public String run(String payload) {
            return "echo:" + payload;
        }
    }

    static final class DuplicateStringBean extends StringBean {}

    static final class FailingBean {
        @TikeoProcessor("demo.fail")
        public void run(TaskContext ignored) {
            throw new IllegalStateException("boom");
        }
    }

    static final class ScriptPrefixedBean {
        @TikeoProcessor("script:shell")
        public void run(String payload) {}
    }

    static final class PluginBean {
        @TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, pluginType = "sql")
        public void run(String payload) {}
    }

    static final class MissingPluginTypeBean {
        @TikeoProcessor(value = "billing.missing", kind = TikeoProcessorKind.PLUGIN)
        public void run(String payload) {}
    }
}
