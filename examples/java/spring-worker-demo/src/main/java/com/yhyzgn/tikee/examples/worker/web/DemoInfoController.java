package com.yhyzgn.tikee.examples.worker.web;

import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import java.util.List;
import lombok.RequiredArgsConstructor;
import org.springframework.beans.factory.ObjectProvider;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/** HTTP surface for checking the standard Spring Boot demo application. */
@RestController
@RequestMapping("/demo")
@RequiredArgsConstructor
public final class DemoInfoController {
    private final TikeeProcessorRegistry registry;
    private final ObjectProvider<TikeeWorkerClient> workerClient;

    @GetMapping("/health")
    public DemoHealth health() {
        TikeeWorkerClient client = workerClient.getIfAvailable();
        boolean connected = client != null && client.connected();
        return new DemoHealth(connected ? "ok" : "disconnected", client == null ? null : client.workerId(), connected, processors());
    }

    @GetMapping("/processors")
    public List<String> processors() {
        return registry.handlers().keySet().stream().sorted().toList();
    }

    public record DemoHealth(String status, String workerId, boolean connected, List<String> processors) {}
}
