package net.tikeo.management.client;

/**
 * Runtime exception raised by the tikeo management client.
 */
public class TikeoManagementException extends RuntimeException {
    public TikeoManagementException(String message) {
        super(message);
    }

    public TikeoManagementException(String message, Throwable cause) {
        super(message, cause);
    }
}
