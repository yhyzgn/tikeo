package net.tikeo.examples.worker.web;

import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.boot.autoconfigure.TikeoWorkerProperties;
import net.tikeo.worker.client.TikeoWorkerClient;
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
@ConditionalOnProperty(prefix = "tikeo.worker", name = "enabled", havingValue = "true", matchIfMissing = true)
public final class DemoInfoController {
    private final TikeoProcessorRegistry registry;
    private final TikeoWorkerClient workerClient;
    private final TikeoWorkerProperties workerProperties;

    @GetMapping("/health")
    public DemoHealth health() {
        boolean connected = workerClient.connected();
        return new DemoHealth(
                connected ? "ok" : "disconnected",
                workerClient.workerId(),
                connected,
                workerProperties.getNamespace(),
                workerProperties.getApp(),
                workerProperties.getLabels().getOrDefault("worker_pool", ""),
                workerProperties.getClientInstanceId(),
                processors());
    }

    @GetMapping("/processors")
    public List<String> processors() {
        return registry.handlers().keySet().stream().sorted().toList();
    }

    public record DemoHealth(
            String status,
            String workerId,
            boolean connected,
            String namespace,
            String app,
            String workerPool,
            String clientInstanceId,
            List<String> processors) {}
}
