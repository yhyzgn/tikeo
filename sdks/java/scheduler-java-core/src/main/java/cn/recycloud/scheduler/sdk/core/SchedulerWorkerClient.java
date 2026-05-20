package cn.recycloud.scheduler.sdk.core;

/**
 * Active outbound Worker Tunnel client contract.
 */
public interface SchedulerWorkerClient extends AutoCloseable {
    /**
     * Connect to scheduler and register this worker.
     */
    void start();

    /**
     * Stop the active outbound connection.
     */
    @Override
    void close();
}
