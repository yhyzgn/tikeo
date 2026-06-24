package net.tikeo.spring.processor;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TikeoProcessor;
import net.tikeo.processor.TikeoProcessorKind;
import net.tikeo.processor.TikeoPluginType;
import net.tikeo.spring.worker.SpringTikeoTaskProcessor;
import net.tikeo.worker.WorkerCapabilitySet;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;

class TikeoProcessorRegistryTest {
    @Test
    void invokesTaskContextMethod() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new ContextBean(), "contextBean");

        TaskOutcome outcome = registry.invoke("demo.context", context("demo.context", "hello"));

        Assertions.assertThat(outcome.success()).isTrue();
        Assertions.assertThat(outcome.message()).isEqualTo("instance-1:hello");
    }

    @Test
    void invokesStringPayloadMethod() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        TaskOutcome outcome = registry.invoke("demo.string", context("demo.string", "hello"));

        Assertions.assertThat(outcome.success()).isTrue();
        Assertions.assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    @Test
    void mapsExceptionsToFailedOutcome() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");

        TaskOutcome outcome = registry.invoke("demo.fail", context("demo.fail", "hello"));

        Assertions.assertThat(outcome.success()).isFalse();
        Assertions.assertThat(outcome.message()).isEqualTo("boom");
    }

    @Test
    void processorExceptionsEmitStackTraceToTaskLogs() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");
        List<String> logs = new ArrayList<>();

        TaskOutcome outcome = registry.invoke("demo.fail", new TaskContext("demo.fail", "demo.fail", "instance-1", "hello".getBytes(StandardCharsets.UTF_8), (level, message) -> logs.add(level + ":" + message)));

        Assertions.assertThat(outcome.success()).isFalse();
        Assertions.assertThat(outcome.message()).isEqualTo("boom");
        Assertions.assertThat(logs).anySatisfy(line -> {
            Assertions.assertThat(line).startsWith("error:java.lang.IllegalStateException: boom");
            Assertions.assertThat(line).contains("FailingBean.run");
        });
    }

    @Test
    void rejectsDuplicateProcessorNames() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        Assertions.assertThatThrownBy(() -> registry.postProcessAfterInitialization(new DuplicateStringBean(), "duplicate"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("duplicate tikeo processor name");
    }

    @Test
    void exposesOnlySdkProcessorCapabilities() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        registry.postProcessAfterInitialization(new PluginBean(), "pluginBean");

        Assertions.assertThat(registry.workerCapabilities().normalProcessors())
                .extracting(WorkerCapabilitySet.Processor::name)
                .containsExactly("demo.string");
        Assertions.assertThat(registry.workerCapabilities().pluginProcessors())
                .anySatisfy(plugin -> {
                    Assertions.assertThat(plugin.type()).isEqualTo("sql");
                    Assertions.assertThat(plugin.processorNames()).containsExactly("billing.sql-sync");
                });
    }

    @Test
    void rejectsPluginProcessorWithoutPluginType() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();

        Assertions.assertThatThrownBy(() -> registry.postProcessAfterInitialization(new MissingPluginTypeBean(), "pluginBean"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("requires non-NONE pluginType");
    }

    @Test
    void rejectsScriptPrefixedProcessorAnnotations() {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();

        Assertions.assertThatThrownBy(() -> registry.postProcessAfterInitialization(new ScriptPrefixedBean(), "scriptBean"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("@TikeoProcessor is reserved for normal processors");
    }

    @Test
    void routesThroughSpringTaskProcessorByJobId() throws Exception {
        TikeoProcessorRegistry registry = new TikeoProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        SpringTikeoTaskProcessor processor = new SpringTikeoTaskProcessor(registry);

        TaskOutcome outcome = processor.process(new TaskContext("job-1", "demo.string", "instance-1", "payload".getBytes(StandardCharsets.UTF_8)));

        Assertions.assertThat(outcome).isEqualTo(new TaskOutcome(true, "echo:payload"));
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
        @TikeoProcessor(value = "billing.sql-sync", kind = TikeoProcessorKind.PLUGIN, pluginType = TikeoPluginType.SQL)
        public void run(String payload) {}
    }

    static final class MissingPluginTypeBean {
        @TikeoProcessor(value = "billing.missing", kind = TikeoProcessorKind.PLUGIN)
        public void run(String payload) {}
    }
}
