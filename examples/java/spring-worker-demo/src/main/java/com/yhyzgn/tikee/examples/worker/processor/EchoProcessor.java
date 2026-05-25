package com.yhyzgn.tikee.examples.worker.processor;

import com.yhyzgn.tikee.core.TikeeProcessor;
import org.springframework.stereotype.Component;

/** Example processor used by both live demo and unit tests. */
@Component
public final class EchoProcessor {
    @TikeeProcessor("demo.echo")
    public String echo(String payload) {
        return "echo:" + payload;
    }
}
