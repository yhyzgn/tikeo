package net.tikeo.examples.worker.lifecycle;

import net.tikeo.boot.autoconfigure.TikeoWorkerProperties;
import net.tikeo.spring.processor.TikeoProcessorRegistry;
import net.tikeo.worker.client.TikeoWorkerClient;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.autoconfigure.condition.ConditionalOnBean;
import org.springframework.boot.context.event.ApplicationReadyEvent;
import org.springframework.context.event.EventListener;
import org.springframework.stereotype.Component;

/** Logs the Java demo worker identity and registered processors after Spring Boot is ready. */
@Slf4j
@Component
@ConditionalOnBean(TikeoWorkerClient.class)
@RequiredArgsConstructor
public final class DemoWorkerStartupLogger {
    private final TikeoWorkerClient workerClient;
    private final TikeoWorkerProperties workerProperties;
    private final TikeoProcessorRegistry registry;

    @EventListener(ApplicationReadyEvent.class)
    public void logWorkerReady() {
        log.info("Java worker demo ready endpoint={} dryRun={} workerId={} connected={} processors={}",
                workerProperties.getEndpoint(),
                workerProperties.isDryRun(),
                workerClient.workerId(),
                workerClient.connected(),
                registry.handlers().keySet().stream().sorted().toList());
        if (!workerProperties.isDryRun() && !workerClient.connected()) {
            log.warn("Java worker demo is not registered yet. Check that tikeo server Worker Tunnel is reachable at {}.",
                    workerProperties.getEndpoint());
        }
    }
}
