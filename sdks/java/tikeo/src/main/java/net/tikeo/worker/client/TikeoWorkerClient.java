package net.tikeo.worker.client;

/**
 * Active outbound Worker Tunnel client contract.
 */
public interface TikeoWorkerClient extends AutoCloseable {
    /**
     * Connect to tikeo and register this worker.
     */
    void start();

    /**
     * @return server-assigned authoritative worker id after registration, or {@code null} before registration
     */
    default String workerId() {
        return null;
    }

    /**
     * @return {@code true} when the client currently has an open registered Worker Tunnel
     */
    default boolean connected() {
        return workerId() != null;
    }

    /**
     * Emit one task log message.
     *
     * @param instanceId job instance id
     * @param level log level
     * @param message log message
     */
    default void emitLog(String instanceId, String level, String message) {
        throw new UnsupportedOperationException("task log emission is not supported by this client");
    }

    /**
     * Stop the active outbound connection.
     */
    @Override
    void close();
}
