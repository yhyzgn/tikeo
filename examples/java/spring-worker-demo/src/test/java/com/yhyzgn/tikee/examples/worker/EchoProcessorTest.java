package com.yhyzgn.tikee.examples.worker;

import static org.assertj.core.api.Assertions.assertThat;

import com.yhyzgn.tikee.examples.worker.processor.EchoProcessor;
import org.junit.jupiter.api.Test;

class EchoProcessorTest {
    @Test
    void echoesPayloadWithDemoPrefix() {
        EchoProcessor processor = new EchoProcessor();

        assertThat(processor.echo("hello")).isEqualTo("echo:hello");
    }
}
