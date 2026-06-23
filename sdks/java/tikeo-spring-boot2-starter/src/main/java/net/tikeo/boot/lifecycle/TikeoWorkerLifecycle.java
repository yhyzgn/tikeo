package net.tikeo.boot.lifecycle;

import net.tikeo.worker.client.TikeoWorkerClient;
import net.tikeo.boot.autoconfigure.TikeoWorkerProperties;
import java.util.concurrent.atomic.AtomicBoolean;
import lombok.RequiredArgsConstructor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.context.SmartLifecycle;

/**
 * Spring lifecycle bridge that owns the outbound tikeo Worker Tunnel connection.
 */
@RequiredArgsConstructor
public final class TikeoWorkerLifecycle implements SmartLifecycle {
    private static final Logger log = LoggerFactory.getLogger(TikeoWorkerLifecycle.class);

    private final TikeoWorkerClient client;
    private final TikeoWorkerProperties properties;
    private final AtomicBoolean running = new AtomicBoolean(false);

    @Override
    public void start() {
        if (running.compareAndSet(false, true)) {
            try {
                client.start();
            } catch (RuntimeException error) {
                running.set(false);
                log.warn("[tikeo.worker] worker tunnel startup failed; application startup will continue and the worker can reconnect when Tikeo is available", error);
            }
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
