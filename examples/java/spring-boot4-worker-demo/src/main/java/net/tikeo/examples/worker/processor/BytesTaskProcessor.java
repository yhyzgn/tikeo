package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TikeoProcessor;
import java.nio.charset.StandardCharsets;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Raw byte payload task example. */
@Slf4j
@Component
public final class BytesTaskProcessor {
    @TikeoProcessor("demo.bytes")
    public String bytes(TaskContext context, byte[] payload) {
        String text = new String(payload, StandardCharsets.UTF_8);
        log.info("[demo.bytes] received bytes payload='{}' length={}", text, payload.length);
        String result = "bytes:" + text;
        log.info("[demo.bytes] completed result='{}'", result);
        return result;
    }
}
