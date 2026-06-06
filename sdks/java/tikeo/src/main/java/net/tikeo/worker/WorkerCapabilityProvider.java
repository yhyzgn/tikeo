package net.tikeo.worker;

/** Provides structured worker capabilities for registration. */
@FunctionalInterface
public interface WorkerCapabilityProvider {
    WorkerCapabilitySet workerCapabilities();
}
