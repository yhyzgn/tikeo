package com.yhyzgn.tikee.examples.worker;

import com.yhyzgn.tikee.sdk.core.TikeeProcessor;
import com.yhyzgn.tikee.sdk.core.TikeeWorkerClient;
import lombok.RequiredArgsConstructor;
import org.springframework.boot.CommandLineRunner;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.stereotype.Component;

@SpringBootApplication
public class SpringWorkerDemoApplication {
    public static void main(String[] args) {
        SpringApplication.run(SpringWorkerDemoApplication.class, args);
    }

    @TikeeProcessor("demo.echo")
    public String echo(String payload) {
        return "echo:" + payload;
    }

    @Component
    @RequiredArgsConstructor
    static class DemoRunner implements CommandLineRunner {
        private final TikeeWorkerClient client;

        @Override
        public void run(String... args) {
            client.start();
            System.out.println("Spring worker demo started with tikee worker client: "
                    + client.getClass().getSimpleName() + ", workerId=" + client.workerId());
            client.close();
        }
    }
}
