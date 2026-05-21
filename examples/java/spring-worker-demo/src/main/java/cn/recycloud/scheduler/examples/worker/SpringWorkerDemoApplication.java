package cn.recycloud.scheduler.examples.worker;

import cn.recycloud.scheduler.sdk.core.SchedulerProcessor;
import cn.recycloud.scheduler.sdk.core.SchedulerWorkerClient;
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

    @SchedulerProcessor("demo.echo")
    public String echo(String payload) {
        return "echo:" + payload;
    }

    @Component
    @RequiredArgsConstructor
    static class DemoRunner implements CommandLineRunner {
        private final SchedulerWorkerClient client;

        @Override
        public void run(String... args) {
            client.start();
            System.out.println("Spring worker demo started with scheduler worker client: "
                    + client.getClass().getSimpleName() + ", workerId=" + client.workerId());
            client.close();
        }
    }
}
