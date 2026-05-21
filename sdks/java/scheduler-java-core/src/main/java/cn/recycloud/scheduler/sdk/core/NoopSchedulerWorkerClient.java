package cn.recycloud.scheduler.sdk.core;

import java.util.Objects;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * Dry-run client for demos/tests that should not open a live Worker Tunnel.
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
    public String workerId() {
        return running.get() ? "dry-run-" + registration.clientInstanceId() : null;
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
