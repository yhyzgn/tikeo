package com.yhyzgn.tikee.boot.lifecycle;

import com.yhyzgn.tikee.worker.client.TikeeWorkerClient;
import com.yhyzgn.tikee.boot.autoconfigure.TikeeWorkerProperties;
import java.util.concurrent.atomic.AtomicBoolean;
import lombok.RequiredArgsConstructor;
import org.springframework.context.SmartLifecycle;

/**
 * Spring lifecycle bridge that owns the outbound tikee Worker Tunnel connection.
 */
@RequiredArgsConstructor
public final class TikeeWorkerLifecycle implements SmartLifecycle {
    private final TikeeWorkerClient client;
    private final TikeeWorkerProperties properties;
    private final AtomicBoolean running = new AtomicBoolean(false);

    @Override
    public void start() {
        if (running.compareAndSet(false, true)) {
            client.start();
        }
    }

    @Override
    public void stop() {
        if (running.compareAndSet(true, false)) {
            client.close();
        }
    }

    @Override
    public boolean isRunning() {
        return running.get();
    }

    @Override
    public boolean isAutoStartup() {
        return properties.isAutoStartup();
    }
}
