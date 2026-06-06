package net.tikeo.boot.lifecycle;

import net.tikeo.worker.client.TikeoWorkerClient;
import net.tikeo.boot.autoconfigure.TikeoWorkerProperties;
import java.util.concurrent.atomic.AtomicBoolean;
import lombok.RequiredArgsConstructor;
import org.springframework.context.SmartLifecycle;

/**
 * Spring lifecycle bridge that owns the outbound tikeo Worker Tunnel connection.
 */
@RequiredArgsConstructor
public final class TikeoWorkerLifecycle implements SmartLifecycle {
    private final TikeoWorkerClient client;
    private final TikeoWorkerProperties properties;
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
