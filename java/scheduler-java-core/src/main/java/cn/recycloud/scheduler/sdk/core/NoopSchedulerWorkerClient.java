package cn.recycloud.scheduler.sdk.core;

import java.util.Objects;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * Placeholder client used until the Java gRPC Worker Tunnel implementation lands.
 */
public final class NoopSchedulerWorkerClient implements SchedulerWorkerClient {
    private final WorkerRegistration registration;
    private final AtomicBoolean running = new AtomicBoolean(false);

    public NoopSchedulerWorkerClient(WorkerRegistration registration) {
        this.registration = Objects.requireNonNull(registration, "registration");
    }

    public WorkerRegistration registration() {
        return registration;
    }

    public boolean running() {
        return running.get();
    }

    @Override
    public void start() {
        running.set(true);
    }

    @Override
    public void close() {
        running.set(false);
    }
}
