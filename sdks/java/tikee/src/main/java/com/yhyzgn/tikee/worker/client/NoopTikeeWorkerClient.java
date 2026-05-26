package com.yhyzgn.tikee.worker.client;

import com.yhyzgn.tikee.worker.WorkerRegistration;
import java.util.concurrent.atomic.AtomicBoolean;
import lombok.NonNull;
import lombok.RequiredArgsConstructor;

/**
 * Dry-run client for demos/tests that should not open a live Worker Tunnel.
 */
@RequiredArgsConstructor
public final class NoopTikeeWorkerClient implements TikeeWorkerClient {
    @NonNull
    private final WorkerRegistration registration;
    private final AtomicBoolean running = new AtomicBoolean(false);

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
    public boolean connected() {
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
