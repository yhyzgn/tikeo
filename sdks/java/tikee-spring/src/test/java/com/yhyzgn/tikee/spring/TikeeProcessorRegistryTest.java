package com.yhyzgn.tikee.spring;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.yhyzgn.tikee.core.TikeeProcessor;
import com.yhyzgn.tikee.core.TaskContext;
import com.yhyzgn.tikee.core.TaskOutcome;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class TikeeProcessorRegistryTest {
    @Test
    void invokesTaskContextMethod() {
        TikeeProcessorRegistry registry = new TikeeProcessorRegistry();
        registry.postProcessAfterInitialization(new ContextBean(), "contextBean");

        TaskOutcome outcome = registry.invoke("demo.context", context("demo.context", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("instance-1:hello");
    }

    @Test
    void invokesStringPayloadMethod() {
        TikeeProcessorRegistry registry = new TikeeProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        TaskOutcome outcome = registry.invoke("demo.string", context("demo.string", "hello"));

        assertThat(outcome.success()).isTrue();
        assertThat(outcome.message()).isEqualTo("echo:hello");
    }

    @Test
    void mapsExceptionsToFailedOutcome() {
        TikeeProcessorRegistry registry = new TikeeProcessorRegistry();
        registry.postProcessAfterInitialization(new FailingBean(), "failingBean");

        TaskOutcome outcome = registry.invoke("demo.fail", context("demo.fail", "hello"));

        assertThat(outcome.success()).isFalse();
        assertThat(outcome.message()).isEqualTo("boom");
    }

    @Test
    void rejectsDuplicateProcessorNames() {
        TikeeProcessorRegistry registry = new TikeeProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");

        assertThatThrownBy(() -> registry.postProcessAfterInitialization(new DuplicateStringBean(), "duplicate"))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("duplicate tikee processor name");
    }

    @Test
    void routesThroughSpringTaskProcessorByJobId() throws Exception {
        TikeeProcessorRegistry registry = new TikeeProcessorRegistry();
        registry.postProcessAfterInitialization(new StringBean(), "stringBean");
        SpringTikeeTaskProcessor processor = new SpringTikeeTaskProcessor(registry);

        TaskOutcome outcome = processor.process(new TaskContext("job-1", "demo.string", "instance-1", "payload".getBytes(StandardCharsets.UTF_8)));

        assertThat(outcome).isEqualTo(new TaskOutcome(true, "echo:payload"));
    }

    private static TaskContext context(String jobId, String payload) {
        return new TaskContext(jobId, "instance-1", payload.getBytes(StandardCharsets.UTF_8));
    }

    static final class ContextBean {
        @TikeeProcessor("demo.context")
        public TaskOutcome run(TaskContext context) {
            return new TaskOutcome(true, context.instanceId() + ":" + new String(context.payload(), StandardCharsets.UTF_8));
        }
    }

    static class StringBean {
        @TikeeProcessor("demo.string")
        public String run(String payload) {
            return "echo:" + payload;
        }
    }

    static final class DuplicateStringBean extends StringBean {}

    static final class FailingBean {
        @TikeeProcessor("demo.fail")
        public void run(TaskContext ignored) {
            throw new IllegalStateException("boom");
        }
    }
}
