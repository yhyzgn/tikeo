package com.yhyzgn.tikee.sdk.core;

/**
 * Runtime exception raised by the Java Worker Tunnel client.
 */
public class WorkerClientException extends RuntimeException {
    public WorkerClientException(String message) {
        super(message);
    }

    public WorkerClientException(String message, Throwable cause) {
        super(message, cause);
    }
}
