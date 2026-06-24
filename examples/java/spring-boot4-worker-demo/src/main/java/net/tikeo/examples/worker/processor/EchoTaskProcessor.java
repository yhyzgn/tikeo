package net.tikeo.examples.worker.processor;

import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TikeoProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** API-triggered success example. */
@Slf4j
@Component
public final class EchoTaskProcessor {
    @TikeoProcessor(value = "demo.echo", description = "回显输入 payload 的普通执行器")
    public String echo(TaskContext context, String payload) {
        log.info("[demo.echo] received payload='{}'", payload);
        String result = "echo:" + payload;
        log.info("[demo.echo] completed result='{}'", result);
        return result;
    }
}
