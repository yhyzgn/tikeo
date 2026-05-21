package cn.recycloud.scheduler.sdk.spring;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import cn.recycloud.scheduler.sdk.core.SchedulerProcessor;
import cn.recycloud.scheduler.sdk.core.TaskContext;
import cn.recycloud.scheduler.sdk.core.TaskOutcome;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class SchedulerProcessorRegistryTest {
    @Test
    void invokesTaskContextMethod() {
        SchedulerProcessorRegistry registry = new SchedulerProcessorRegistry();
        registry.postProcessAfterInitialization(new ContextBean(), "contextBean");

        TaskOutcome outcome = registry.invoke("demo.context", context("demo.context", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("instance-1:hello");
    }

    @Test
    void invokesStringPayloadMethod() {
        SchedulerProcessorRegistry registry = new SchedulerProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        TaskOutcome outcome = registry.invoke("demo.string", context("demo.string", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    @Test
    void mapsExceptionsToFailedOutcome() {
        SchedulerProcessorRegistry registry = new SchedulerProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");

        TaskOutcome outcome = registry.invoke("demo.fail", context("demo.fail", "hello"));

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("boom");
    }

    @Test
    void rejectsDuplicateProcessorNames() {
        SchedulerProcessorRegistry registry = new SchedulerProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        assertThatThrownBy(() -> registry.postProcessAfterInitialization(new DuplicateStringBean(), "duplicate"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("duplicate scheduler processor name");
    }

    @Test
    void routesThroughSpringTaskProcessorByJobId() throws Exception {
        SchedulerProcessorRegistry registry = new SchedulerProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        SpringSchedulerTaskProcessor processor = new SpringSchedulerTaskProcessor(registry);

        TaskOutcome outcome = processor.process(new TaskContext("job-1", "demo.string", "instance-1", "payload".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome).isEqualTo(new TaskOutcome(true, "echo:payload"));
    }

    private static TaskContext context(String jobId, String payload) {
        return new TaskContext(jobId, "instance-1", payload.getBytes(StandardCharsets.UTF_8));
    }

    static final class ContextBean {
        @SchedulerProcessor("demo.context")
        public TaskOutcome run(TaskContext context) {
            return new TaskOutcome(true, context.instanceId() + ":" + new String(context.payload(), StandardCharsets.UTF_8));
        }
    }

    static class StringBean {
        @SchedulerProcessor("demo.string")
        public String run(String payload) {
            return "echo:" + payload;
        }
    }

    static final class DuplicateStringBean extends StringBean {}

    static final class FailingBean {
        @SchedulerProcessor("demo.fail")
        public void run(TaskContext ignored) {
            throw new IllegalStateException("boom");
        }
    }
}
