package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.processor.TikeeProcessor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/** API-triggered success example. */
@Slf4j
@Component
public final class EchoTaskProcessor {
    @TikeeProcessor("demo.echo")
    public String echo(String payload) {
        log.info("[demo.echo] received payload='{}'", payload);
        String result = "echo:" + payload;
        log.info("[demo.echo] completed result='{}'", result);
        return result;
    }
}
