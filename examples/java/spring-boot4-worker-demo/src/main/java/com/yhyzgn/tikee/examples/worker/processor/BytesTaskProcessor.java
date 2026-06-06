package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TikeeProcessor;
import java.nio.charset.StandardCharsets;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** Raw byte payload task example. */
@Slf4j
@Component
public final class BytesTaskProcessor {
    @TikeeProcessor("demo.bytes")
    public String bytes(TaskContext context, byte[] payload) {
        String text = new String(payload, StandardCharsets.UTF_8);
        log.info("[demo.bytes] received bytes payload='{}' length={}", text, payload.length);
        context.logInfo("[demo.bytes] received bytes payload='" + text + "' length=" + payload.length);
        String result = "bytes:" + text;
        log.info("[demo.bytes] completed result='{}'", result);
        context.logInfo("[demo.bytes] completed result='" + result + "'");
        return result;
    }
}
