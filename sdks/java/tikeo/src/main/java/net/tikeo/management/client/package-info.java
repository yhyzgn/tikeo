/**
 * Java management-client implementations.
 *
 * <p>{@link net.tikeo.management.client.TikeoJobClient} provides job CRUD and explicit trigger
 * operations for one namespace/app scope. {@link net.tikeo.management.client.HttpTikeoJobClient}
 * uses the JDK HTTP client and the {@code x-tikeo-api-key} header.
 *
 * <p><strong>Usage:</strong> construct the client with the server endpoint, API key, namespace, and app,
 * then create API, cron, plugin, or script jobs with typed request records.
 */
package net.tikeo.management.client;
