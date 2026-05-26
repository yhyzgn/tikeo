package com.yhyzgn.tikee.examples.worker.lifecycle;

import com.yhyzgn.tikee.boot.autoconfigure.TikeeWorkerProperties;
import com.yhyzgn.tikee.spring.processor.TikeeProcessorRegistry;
import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.autoconfigure.condition.ConditionalOnBean;
import org.springframework.boot.context.event.ApplicationReadyEvent;
import org.springframework.context.event.EventListener;
import org.springframework.stereotype.Component;

/** Logs the Java demo worker identity and registered processors after Spring Boot is ready. */
@Slf4j
@Component
@ConditionalOnBean(TikeeWorkerClient.class)
@RequiredArgsConstructor
public final class DemoWorkerStartupLogger {
    private final TikeeWorkerClient workerClient;
    private final TikeeWorkerProperties workerProperties;
    private final TikeeProcessorRegistry registry;

    @EventListener(ApplicationReadyEvent.class)
    public void logWorkerReady() {
        log.info("Java worker demo ready endpoint={} dryRun={} workerId={} connected={} processors={}",
                workerProperties.getEndpoint(),
                workerProperties.isDryRun(),
                workerClient.workerId(),
                workerClient.connected(),
                registry.handlers().keySet().stream().sorted().toList());
        if (!workerProperties.isDryRun() && !workerClient.connected()) {
            log.warn("Java worker demo is not registered yet. Check that tikee server Worker Tunnel is reachable at {}.",
                    workerProperties.getEndpoint());
        }
    }
}
