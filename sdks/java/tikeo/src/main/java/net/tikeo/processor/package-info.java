/**
 * Public task processor contracts for Java workers.
 *
 * <p>Applications implement {@link net.tikeo.processor.TaskProcessor} or annotate Spring beans with
 * {@link net.tikeo.processor.TikeoProcessor} to expose executor names to tikeo. Processor names are
 * matched through structured worker capabilities, not by parsing free-form strings.
 *
 * <p><strong>Usage:</strong> emit task output with {@link net.tikeo.processor.TaskContext#logInfo(String)}
 * and {@link net.tikeo.processor.TaskContext#logError(String)} so logs are attached precisely to the
 * current job instance.
 *
 * <p><strong>Operational cautions:</strong> do not write secrets or full payloads to task logs. Use SDK
 * diagnostics, backed by SLF4J, for Worker Tunnel lifecycle issues and reserve task logs for
 * operator-facing execution evidence.
 */
package net.tikeo.processor;
