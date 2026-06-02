package com.yhyzgn.tikee.examples.worker.web;

import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import java.util.List;
import lombok.RequiredArgsConstructor;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/** HTTP surface for checking the standard Spring Boot demo application. */
@RestController
@RequestMapping("/demo")
@RequiredArgsConstructor
@ConditionalOnProperty(prefix = "tikee.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
public final class DemoInfoController {
    private final TikeeProcessorRegistry registry;
    private final TikeeWorkerClient workerClient;

    @GetMapping("/health")
    public DemoHealth health() {
        boolean connected = workerClient.connected();
        return new DemoHealth(connected ? "ok" : "disconnected", workerClient.workerId(), connected, processors());
    }

    @GetMapping("/processors")
    public List<String> processors() {
        return registry.handlers().keySet().stream().sorted().toList();
    }

    public record DemoHealth(String status, String workerId, boolean connected, List<String> processors) {}
}
