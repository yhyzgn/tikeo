/**
 * Active outbound Worker Tunnel clients.
 *
 * <p>{@link net.tikeo.worker.client.GrpcTikeoWorkerClient} opens the gRPC tunnel, registers the
 * worker, renews leases, receives assignments, emits precise task logs, and reports results.
 *
 * <p><strong>Usage:</strong> create a client with a {@link net.tikeo.worker.WorkerRegistration}, start it
 * during application startup, and close it during shutdown. Spring users normally rely on the
 * starter lifecycle bean instead of constructing the client manually.
 *
 * <p><strong>Operational cautions:</strong> Java SDK diagnostics use SLF4J. Configure Logback or Log4j2 in
 * the host application for console plus file output; the SDK intentionally does not force a logging
 * backend. Keep the default INFO level and enable DEBUG only for short troubleshooting windows.
 */
package net.tikeo.worker.client;
