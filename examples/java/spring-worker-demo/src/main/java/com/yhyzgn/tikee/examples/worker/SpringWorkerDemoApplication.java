package com.yhyzgn.tikee.examples.worker;

import com.yhyzgn.tikee.core.TikeeProcessor;
import com.yhyzgn.tikee.core.TikeeWorkerClient;
import jakarta.annotation.PreDestroy;
import java.util.concurrent.CountDownLatch;
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
        private final CountDownLatch stopSignal = new CountDownLatch(1);

        @Override
        public void run(String... args) throws InterruptedException {
            System.out.println("Spring worker demo started with tikee worker client: "
                    + client.getClass().getSimpleName() + ", workerId=" + client.workerId());
            stopSignal.await();
        }

        @PreDestroy
        public void stop() {
            stopSignal.countDown();
        }
    }
}
