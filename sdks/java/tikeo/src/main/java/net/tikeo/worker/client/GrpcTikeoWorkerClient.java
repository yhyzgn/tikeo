package net.tikeo.worker.client;

import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.stub.StreamObserver;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.net.URI;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Consumer;
import net.tikeo.logging.TikeoTaskLogScope;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TaskProcessor;
import net.tikeo.script.ScriptRunnerKind;
import net.tikeo.script.ScriptRunnerPolicy;
import net.tikeo.script.ScriptRunnerRegistry;
import net.tikeo.script.ScriptRunnerTask;
import net.tikeo.script.ScriptSandboxBackend;
import net.tikeo.wasm.WasmRunnerPolicy;
import net.tikeo.wasm.WasmRunnerRegistry;
import net.tikeo.wasm.WasmRunnerTask;
import net.tikeo.worker.WorkerCapabilityProvider;
import net.tikeo.worker.WorkerCapabilitySet;
import net.tikeo.worker.WorkerRegistration;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import tikeo.worker.v1.Worker;
import tikeo.worker.v1.WorkerTunnelServiceGrpc;

/**
 * gRPC implementation of the active outbound Worker Tunnel client.
 */
public final class GrpcTikeoWorkerClient implements TikeoWorkerClient {

    private static final Logger log = LoggerFactory.getLogger(
        GrpcTikeoWorkerClient.class
    );
    private static final Duration DEFAULT_START_TIMEOUT = Duration.ofSeconds(
        10
    );
    private static final Duration DEFAULT_RECONNECT_INITIAL_DELAY =
        Duration.ofSeconds(1);
    private static final Duration DEFAULT_RECONNECT_MAX_DELAY =
        Duration.ofSeconds(30);

    private final WorkerRegistration registration;
    private final ManagedChannel channel;
    private final boolean ownsChannel;
    private final TaskProcessor processor;
    private final ScriptRunnerRegistry scriptRunners;
    private final WasmRunnerRegistry wasmRunners;
    private final Duration heartbeatInterval;
    private final Duration startTimeout;
    private final Duration reconnectInitialDelay;
    private final Duration reconnectMaxDelay;
    private final Consumer<Worker.DispatchTask> dispatchObserver;
    private final ScheduledExecutorService tikeo;
    private final ExecutorService processorExecutor;
    private final Object outboundLock = new Object();
    private final AtomicReference<String> workerId = new AtomicReference<>();
    private final AtomicReference<String> fencingToken = new AtomicReference<>(
        ""
    );
    private final AtomicReference<
        StreamObserver<Worker.WorkerMessage>
    > outbound = new AtomicReference<>();
    private final AtomicReference<Throwable> terminalError =
        new AtomicReference<>();
    private final AtomicBoolean started = new AtomicBoolean(false);
    private final AtomicBoolean closed = new AtomicBoolean(false);
    private final AtomicBoolean connected = new AtomicBoolean(false);
    private final AtomicBoolean reconnectScheduled = new AtomicBoolean(false);
    private final AtomicLong generation = new AtomicLong(0);
    private final AtomicLong heartbeatSequence = new AtomicLong(0);
    private final AtomicLong logSequence = new AtomicLong(0);
    private final AtomicLong reconnectAttempt = new AtomicLong(0);
    private volatile CountDownLatch registrationLatch = new CountDownLatch(1);
    private volatile ScheduledFuture<?> heartbeatTask;
    private volatile ScheduledFuture<?> reconnectTask;

    /**
     * Create a Worker Tunnel client from endpoint and registration metadata.
     *
     * @param endpoint tikeo Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     */
    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration
    ) {
        this(endpoint, registration, context -> TaskOutcome.succeeded());
    }

    /**
     * Create a Worker Tunnel client with a task processor.
     *
     * @param endpoint tikeo Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     * @param processor task processor
     */
    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration,
        TaskProcessor processor
    ) {
        this(endpoint, registration, processor, Duration.ofSeconds(10));
    }

    /**
     * Create a Worker Tunnel client with a task processor and script runners.
     *
     * @param endpoint tikeo Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     * @param processor task processor
     * @param scriptRunners sandboxed script runner registry
     */
    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration,
        TaskProcessor processor,
        ScriptRunnerRegistry scriptRunners
    ) {
        this(
            endpoint,
            registration,
            processor,
            scriptRunners,
            Duration.ofSeconds(10)
        );
    }

    /**
     * Create a Worker Tunnel client with custom heartbeat interval.
     *
     * @param endpoint tikeo Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     * @param processor task processor
     * @param heartbeatInterval heartbeat interval
     */
    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration,
        TaskProcessor processor,
        Duration heartbeatInterval
    ) {
        this(
            channelForEndpoint(endpoint),
            true,
            registration,
            processor,
            new ScriptRunnerRegistry(),
            new WasmRunnerRegistry(),
            heartbeatInterval,
            DEFAULT_START_TIMEOUT,
            DEFAULT_RECONNECT_INITIAL_DELAY,
            DEFAULT_RECONNECT_MAX_DELAY,
            ignored -> {}
        );
    }

    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration,
        TaskProcessor processor,
        ScriptRunnerRegistry scriptRunners,
        Duration heartbeatInterval
    ) {
        this(
            channelForEndpoint(endpoint),
            true,
            registration,
            processor,
            scriptRunners,
            new WasmRunnerRegistry(),
            heartbeatInterval,
            DEFAULT_START_TIMEOUT,
            DEFAULT_RECONNECT_INITIAL_DELAY,
            DEFAULT_RECONNECT_MAX_DELAY,
            ignored -> {}
        );
    }

    public GrpcTikeoWorkerClient(
        String endpoint,
        WorkerRegistration registration,
        TaskProcessor processor,
        ScriptRunnerRegistry scriptRunners,
        WasmRunnerRegistry wasmRunners,
        Duration heartbeatInterval
    ) {
        this(
            channelForEndpoint(endpoint),
            true,
            registration,
            processor,
            scriptRunners,
            wasmRunners,
            heartbeatInterval,
            DEFAULT_START_TIMEOUT,
            DEFAULT_RECONNECT_INITIAL_DELAY,
            DEFAULT_RECONNECT_MAX_DELAY,
            ignored -> {}
        );
    }

    GrpcTikeoWorkerClient(
        ManagedChannel channel,
        boolean ownsChannel,
        WorkerRegistration registration,
        TaskProcessor processor,
        Duration heartbeatInterval,
        Duration startTimeout,
        Consumer<Worker.DispatchTask> dispatchObserver
    ) {
        this(
            channel,
            ownsChannel,
            registration,
            processor,
            new ScriptRunnerRegistry(),
            new WasmRunnerRegistry(),
            heartbeatInterval,
            startTimeout,
            DEFAULT_RECONNECT_INITIAL_DELAY,
            DEFAULT_RECONNECT_MAX_DELAY,
            dispatchObserver
        );
    }

    GrpcTikeoWorkerClient(
        ManagedChannel channel,
        boolean ownsChannel,
        WorkerRegistration registration,
        TaskProcessor processor,
        Duration heartbeatInterval,
        Duration startTimeout,
        Duration reconnectInitialDelay,
        Duration reconnectMaxDelay,
        Consumer<Worker.DispatchTask> dispatchObserver
    ) {
        this(
            channel,
            ownsChannel,
            registration,
            processor,
            new ScriptRunnerRegistry(),
            new WasmRunnerRegistry(),
            heartbeatInterval,
            startTimeout,
            reconnectInitialDelay,
            reconnectMaxDelay,
            dispatchObserver
        );
    }

    GrpcTikeoWorkerClient(
        ManagedChannel channel,
        boolean ownsChannel,
        WorkerRegistration registration,
        TaskProcessor processor,
        ScriptRunnerRegistry scriptRunners,
        Duration heartbeatInterval,
        Duration startTimeout,
        Duration reconnectInitialDelay,
        Duration reconnectMaxDelay,
        Consumer<Worker.DispatchTask> dispatchObserver
    ) {
        this(
            channel,
            ownsChannel,
            registration,
            processor,
            scriptRunners,
            new WasmRunnerRegistry(),
            heartbeatInterval,
            startTimeout,
            reconnectInitialDelay,
            reconnectMaxDelay,
            dispatchObserver
        );
    }

    GrpcTikeoWorkerClient(
        ManagedChannel channel,
        boolean ownsChannel,
        WorkerRegistration registration,
        TaskProcessor processor,
        ScriptRunnerRegistry scriptRunners,
        WasmRunnerRegistry wasmRunners,
        Duration heartbeatInterval,
        Duration startTimeout,
        Duration reconnectInitialDelay,
        Duration reconnectMaxDelay,
        Consumer<Worker.DispatchTask> dispatchObserver
    ) {
        this.registration = Objects.requireNonNull(
            registration,
            "registration"
        );
        this.channel = Objects.requireNonNull(channel, "channel");
        this.ownsChannel = ownsChannel;
        this.processor = Objects.requireNonNull(processor, "processor");
        this.scriptRunners = Objects.requireNonNull(
            scriptRunners,
            "scriptRunners"
        );
        this.wasmRunners = Objects.requireNonNull(wasmRunners, "wasmRunners");
        this.heartbeatInterval = positiveDuration(
            heartbeatInterval,
            "heartbeatInterval"
        );
        this.startTimeout = positiveDuration(startTimeout, "startTimeout");
        this.reconnectInitialDelay = positiveDuration(
            reconnectInitialDelay,
            "reconnectInitialDelay"
        );
        this.reconnectMaxDelay = positiveDuration(
            reconnectMaxDelay,
            "reconnectMaxDelay"
        );
        this.dispatchObserver = Objects.requireNonNull(
            dispatchObserver,
            "dispatchObserver"
        );
        ScheduledThreadPoolExecutor executor = new ScheduledThreadPoolExecutor(
            1
        );
        executor.setRemoveOnCancelPolicy(true);
        this.tikeo = executor;
        this.processorExecutor = Executors.newCachedThreadPool(runnable -> {
            Thread thread = new Thread(runnable, "tikeo-worker-java-processor");
            thread.setDaemon(true);
            return thread;
        });
    }

    @Override
    public synchronized void start() {
        if (!started.compareAndSet(false, true)) {
            return;
        }
        closed.set(false);
        terminalError.set(null);
        try {
            openTunnelAndRegister();
        } catch (RuntimeException error) {
            terminalError.compareAndSet(null, error);
            markDisconnected();
            scheduleReconnect();
        }
    }

    @Override
    public String workerId() {
        return workerId.get();
    }

    @Override
    public boolean connected() {
        return (
            connected.get() && outbound.get() != null && workerId.get() != null
        );
    }

    @Override
    public void emitLog(String instanceId, String level, String message) {
        String assignedWorkerId = requireConnectedWorkerId();
        send(
            Worker.WorkerMessage.newBuilder()
                .setTaskLog(
                    Worker.TaskLog.newBuilder()
                        .setWorkerId(assignedWorkerId)
                        .setInstanceId(
                            Objects.requireNonNull(instanceId, "instanceId")
                        )
                        .setLevel(Objects.requireNonNullElse(level, "info"))
                        .setMessage(Objects.requireNonNullElse(message, ""))
                        .setSequence(logSequence.incrementAndGet())
                        .build()
                )
                .build()
        );
    }

    @Override
    public synchronized void close() {
        closed.set(true);
        started.set(false);
        Optional.ofNullable(reconnectTask).ifPresent(task -> task.cancel(true));
        reconnectTask = null;
        reconnectScheduled.set(false);
        Optional.ofNullable(heartbeatTask).ifPresent(task -> task.cancel(true));
        heartbeatTask = null;
        connected.set(false);
        StreamObserver<Worker.WorkerMessage> observer = outbound.getAndSet(
            null
        );
        if (observer != null) {
            sendGracefulUnregister(observer);
            observer.onCompleted();
        }
        tikeo.shutdownNow();
        processorExecutor.shutdownNow();
        if (ownsChannel) {
            channel.shutdownNow();
        }
    }

    private synchronized void openTunnelAndRegister() {
        if (closed.get() || connected()) {
            return;
        }
        terminalError.set(null);
        registrationLatch = new CountDownLatch(1);
        WorkerTunnelServiceGrpc.WorkerTunnelServiceStub stub =
            WorkerTunnelServiceGrpc.newStub(channel);
        StreamObserver<Worker.WorkerMessage> requestObserver = stub.openTunnel(
            new ServerObserver()
        );
        outbound.set(requestObserver);
        requestObserver.onNext(registerMessage());
        awaitRegistration();
        startHeartbeat();
        reconnectAttempt.set(0);
    }

    private void startHeartbeat() {
        Optional.ofNullable(heartbeatTask).ifPresent(task ->
            task.cancel(false)
        );
        heartbeatTask = tikeo.scheduleAtFixedRate(
            this::sendHeartbeatSafely,
            heartbeatInterval.toMillis(),
            heartbeatInterval.toMillis(),
            TimeUnit.MILLISECONDS
        );
    }

    private void sendGracefulUnregister(
        StreamObserver<Worker.WorkerMessage> observer
    ) {
        String assignedWorkerId = workerId.get();
        if (assignedWorkerId == null || assignedWorkerId.isBlank()) {
            return;
        }
        observer.onNext(
            Worker.WorkerMessage.newBuilder()
                .setUnregister(
                    Worker.UnregisterWorker.newBuilder()
                        .setWorkerId(assignedWorkerId)
                        .setGeneration(generation.get())
                        .setFencingToken(fencingToken.get())
                        .build()
                )
                .build()
        );
    }

    private static ManagedChannel channelForEndpoint(String endpoint) {
        URI uri = URI.create(Objects.requireNonNull(endpoint, "endpoint"));
        int port = uri.getPort();
        if (port < 0) {
            port = switch (uri.getScheme() == null ? "" : uri.getScheme()) {
                case "https" -> 443;
                default -> 80;
            };
        }
        String host =
            uri.getHost() == null ? uri.getSchemeSpecificPart() : uri.getHost();
        ManagedChannelBuilder<?> builder = ManagedChannelBuilder.forAddress(
            host,
            port
        );
        if (!"https".equalsIgnoreCase(uri.getScheme())) {
            builder.usePlaintext();
        }
        return builder.build();
    }

    private static Duration positiveDuration(Duration duration, String name) {
        Objects.requireNonNull(duration, name);
        if (duration.isZero() || duration.isNegative()) {
            throw new IllegalArgumentException(name + " must be positive");
        }
        return duration;
    }

    private Worker.WorkerMessage registerMessage() {
        Worker.RegisterWorker.Builder builder =
            Worker.RegisterWorker.newBuilder()
                .setClientInstanceId(registration.clientInstanceId())
                .setNamespace(registration.namespace())
                .setApp(registration.app())
                .setCluster(registration.cluster())
                .setRegion(registration.region())
                .addAllCapabilities(registrationCapabilities())
                .setStructuredCapabilities(structuredRegistrationCapabilities())
                .setElection(Worker.WorkerClusterElection.newBuilder()
                    .setEnabled(registration.election().enabled())
                    .setDomain(registration.election().domain())
                    .setPriority(registration.election().priority()))
                .putAllLabels(registration.labels());
        return Worker.WorkerMessage.newBuilder().setRegister(builder).build();
    }

    private List<String> registrationCapabilities() {
        var capabilities = new LinkedHashSet<String>();
        capabilities.addAll(registration.capabilities());
        return List.copyOf(capabilities);
    }

    private Worker.WorkerCapabilities structuredRegistrationCapabilities() {
        WorkerCapabilitySet capabilities = registration.structuredCapabilities();
        if (processor instanceof WorkerCapabilityProvider provider) {
            capabilities = capabilities.merge(provider.workerCapabilities());
        }
        capabilities = capabilities.merge(new WorkerCapabilitySet(
            List.of(),
            List.of(),
            scriptRunners.structuredCapabilities(),
            List.of()
        ));
        capabilities = capabilities.merge(new WorkerCapabilitySet(
            List.of(),
            List.of(),
            wasmRunners.structuredCapabilities(),
            List.of()
        ));
        return toProto(capabilities);
    }

    private static Worker.WorkerCapabilities toProto(WorkerCapabilitySet capabilities) {
        Worker.WorkerCapabilities.Builder builder = Worker.WorkerCapabilities.newBuilder()
            .addAllTags(capabilities.tags());
        for (WorkerCapabilitySet.Processor processor : capabilities.normalProcessors()) {
            builder.addNormalProcessors(Worker.ProcessorCapability.newBuilder()
                .setName(processor.name())
                .setDescription(processor.description()));
        }
        for (WorkerCapabilitySet.ScriptRunner runner : capabilities.scriptRunners()) {
            builder.addScriptRunners(Worker.ScriptRunnerCapability.newBuilder()
                .setLanguage(runner.language())
                .setSandboxBackend(runner.sandboxBackend()));
        }
        for (WorkerCapabilitySet.PluginProcessor plugin : capabilities.pluginProcessors()) {
            Worker.PluginProcessorCapability.Builder pluginBuilder = Worker.PluginProcessorCapability.newBuilder()
                .setType(plugin.type())
                .addAllProcessorNames(plugin.processorNames());
            for (WorkerCapabilitySet.Processor processor : plugin.processors()) {
                pluginBuilder.addProcessors(Worker.ProcessorCapability.newBuilder()
                    .setName(processor.name())
                    .setDescription(processor.description()));
            }
            builder.addPluginProcessors(pluginBuilder);
        }
        return builder.build();
    }

    private void awaitRegistration() {
        try {
            if (
                !registrationLatch.await(
                    startTimeout.toMillis(),
                    TimeUnit.MILLISECONDS
                )
            ) {
                throw new WorkerClientException(
                    "worker registration timed out"
                );
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new WorkerClientException(
                "worker registration interrupted",
                error
            );
        }
        Throwable error = terminalError.get();
        if (error != null && !connected.get()) {
            throw new WorkerClientException(
                "worker tunnel failed during registration",
                error
            );
        }
        requireWorkerId();
    }

    private String requireWorkerId() {
        String assignedWorkerId = workerId.get();
        if (assignedWorkerId == null || assignedWorkerId.isBlank()) {
            throw new WorkerClientException("worker is not registered");
        }
        return assignedWorkerId;
    }

    private String requireConnectedWorkerId() {
        String assignedWorkerId = requireWorkerId();
        if (!connected()) {
            throw new WorkerClientException("worker tunnel is not open");
        }
        return assignedWorkerId;
    }

    private void sendHeartbeatSafely() {
        try {
            String assignedWorkerId = workerId.get();
            if (assignedWorkerId == null || assignedWorkerId.isBlank()) {
                return;
            }
            send(
                Worker.WorkerMessage.newBuilder()
                    .setHeartbeat(
                        Worker.Heartbeat.newBuilder()
                            .setWorkerId(assignedWorkerId)
                            .setSequence(heartbeatSequence.incrementAndGet())
                            .setGeneration(generation.get())
                            .setFencingToken(fencingToken.get())
                            .build()
                    )
                    .build()
            );
        } catch (RuntimeException error) {
            terminalError.compareAndSet(null, error);
            markDisconnected();
            scheduleReconnect();
        }
    }

    private void send(Worker.WorkerMessage message) {
        synchronized (outboundLock) {
            StreamObserver<Worker.WorkerMessage> observer = outbound.get();
            if (observer == null || !connected()) {
                throw new WorkerClientException("worker tunnel is not open");
            }
            observer.onNext(message);
        }
    }

    private void markDisconnected() {
        connected.set(false);
        outbound.set(null);
        Optional.ofNullable(heartbeatTask).ifPresent(task ->
            task.cancel(false)
        );
        heartbeatTask = null;
    }

    private void scheduleReconnect() {
        if (closed.get() || !started.get() || connected()) {
            return;
        }
        if (!reconnectScheduled.compareAndSet(false, true)) {
            return;
        }
        long attempt = reconnectAttempt.incrementAndGet();
        long delayMillis = reconnectDelayMillis(attempt);
        reconnectTask = tikeo.schedule(
            () -> {
                reconnectScheduled.set(false);
                if (closed.get() || !started.get() || connected()) {
                    return;
                }
                try {
                    openTunnelAndRegister();
                } catch (RuntimeException error) {
                    terminalError.compareAndSet(null, error);
                    markDisconnected();
                    scheduleReconnect();
                }
            },
            delayMillis,
            TimeUnit.MILLISECONDS
        );
    }

    private long reconnectDelayMillis(long attempt) {
        long initial = reconnectInitialDelay.toMillis();
        long max = reconnectMaxDelay.toMillis();
        long multiplier = 1L << Math.min(attempt - 1, 10);
        long delay = initial * multiplier;
        if (delay < 0 || delay > max) {
            return max;
        }
        return Math.max(initial, delay);
    }

    private void handleDispatch(Worker.DispatchTask task) {
        dispatchObserver.accept(task);
        processorExecutor.submit(() -> {
            String assignedWorkerId = requireConnectedWorkerId();
            log.info(
                "[tikeo.worker] received task instanceId={} jobId={} processor={}",
                task.getInstanceId(),
                task.getJobId(),
                task.getProcessorName()
            );
            emitTaskLogSafely(
                task,
                assignedWorkerId,
                "info",
                "received task " +
                    task.getInstanceId() +
                    " processor=" +
                    task.getProcessorName(),
                true
            );
            TaskOutcome outcome;
            try {
                outcome = processDispatchedTask(task, assignedWorkerId);
            } catch (Exception error) {
                emitTaskLogSafely(task, assignedWorkerId, "error", stackTrace(error), true);
                outcome = TaskOutcome.failed(errorMessage(error));
            }
            String level = outcome.success() ? "info" : "error";
            log.info(
                "[tikeo.worker] completed task instanceId={} processor={} success={} message={}",
                task.getInstanceId(),
                task.getProcessorName(),
                outcome.success(),
                outcome.message()
            );
            emitTaskLogSafely(
                task,
                assignedWorkerId,
                level,
                "completed task " +
                    task.getInstanceId() +
                    " success=" +
                    outcome.success() +
                    " message=" +
                    outcome.message(),
                true
            );
            send(
                Worker.WorkerMessage.newBuilder()
                    .setTaskResult(
                        Worker.TaskResult.newBuilder()
                            .setWorkerId(assignedWorkerId)
                            .setInstanceId(task.getInstanceId())
                            .setSuccess(outcome.success())
                            .setMessage(outcome.message())
                            .setAssignmentToken(task.getAssignmentToken())
                            .build()
                    )
                    .build()
            );
        });
    }

    private TaskOutcome processDispatchedTask(
        Worker.DispatchTask task,
        String assignedWorkerId
    ) throws Exception {
        if (task.hasProcessorBinding() && task.getProcessorBinding().hasWasm()) {
            return processWasmBinding(task, assignedWorkerId);
        }
        if (task.hasProcessorBinding() && task.getProcessorBinding().hasScript()) {
            return processScriptBinding(task, assignedWorkerId);
        }
        TaskContext context = new TaskContext(
            task.getJobId(),
            task.getProcessorName(),
            task.getInstanceId(),
            task.getPayload().toByteArray(),
            (level, message) -> emitTaskLogSafely(task, assignedWorkerId, level, message, true)
        );
        return TikeoTaskLogScope.captureThrowing(
            task.getJobId(),
            task.getProcessorName(),
            task.getInstanceId(),
            context.logger(),
            () -> processor.process(context)
        );
    }

    private TaskOutcome processWasmBinding(
        Worker.DispatchTask task,
        String assignedWorkerId
    ) {
        Worker.WasmProcessorBinding binding = task
            .getProcessorBinding()
            .getWasm();
        try {
            return wasmRunners
                .runner()
                .orElseThrow(() ->
                    new WorkerClientException("wasm runner is not registered")
                )
                .run(
                    new WasmRunnerTask(
                        binding.getScriptId(),
                        binding.getVersionId(),
                        binding.getVersionNumber(),
                        binding.getModule().toByteArray(),
                        binding.getModuleSha256(),
                        binding.getRuntime(),
                        binding.getEntrypoint(),
                        new WasmRunnerPolicy(
                            binding.getTimeoutMs(),
                            binding.getMaxMemoryBytes(),
                            binding.getFuel(),
                            binding.getAllowNetwork(),
                            binding.getAllowedEnvVarsList()
                        )
                    ),
                    (level, message) -> emitTaskLogSafely(task, assignedWorkerId, level, message, true)
                );
        } catch (Exception error) {
            return TaskOutcome.failed(errorMessage(error));
        }
    }

    private TaskOutcome processScriptBinding(
        Worker.DispatchTask task,
        String assignedWorkerId
    ) {
        Worker.ScriptProcessorBinding binding = task
            .getProcessorBinding()
            .getScript();
        try {
            ScriptRunnerKind kind = ScriptRunnerKind.fromLanguage(
                binding.getLanguage()
            ).orElseThrow(() ->
                new WorkerClientException(
                    "unsupported script language: " + binding.getLanguage()
                )
            );
            return scriptRunners
                .find(kind)
                .orElseThrow(() ->
                    new WorkerClientException(
                        "script runner is not registered for language: " +
                            binding.getLanguage()
                    )
                )
                .run(
                    new ScriptRunnerTask(
                        binding.getScriptId(),
                        binding.getVersionId(),
                        binding.getVersionNumber(),
                        binding.getLanguage(),
                        binding.getContent().toStringUtf8(),
                        binding.getContentSha256(),
                        new ScriptRunnerPolicy(
                            binding.getTimeoutMs(),
                            binding.getMaxMemoryBytes(),
                            binding.getMaxOutputBytes(),
                            binding.getAllowNetwork(),
                            binding.getAllowedNetworkHostsList(),
                            binding.getAllowedEnvVarsList(),
                            binding.getReadOnlyPathsList(),
                            binding.getWritablePathsList(),
                            binding.getSecretRefsList()
                        ),
                        ScriptSandboxBackend.fromValue(
                            binding.getSandboxBackend()
                        )
                    ),
                    (level, message) -> emitTaskLogSafely(task, assignedWorkerId, level, message, true)
                );
        } catch (Exception error) {
            return TaskOutcome.failed(errorMessage(error));
        }
    }

    private void emitTaskLogSafely(
        Worker.DispatchTask task,
        String assignedWorkerId,
        String level,
        String message,
        boolean printLocally
    ) {
        try {
            emitTaskLog(task, assignedWorkerId, level, message, printLocally);
        } catch (RuntimeException error) {
            log.warn(
                "failed to emit task log instanceId={} level={} message={}",
                task.getInstanceId(),
                level,
                message,
                error
            );
        }
    }

    private void emitTaskLog(
        Worker.DispatchTask task,
        String assignedWorkerId,
        String level,
        String message,
        boolean printLocally
    ) {
        if (printLocally) {
            printTaskLogLocally(level, message);
        }
        send(
            Worker.WorkerMessage.newBuilder()
                .setTaskLog(
                    Worker.TaskLog.newBuilder()
                        .setWorkerId(assignedWorkerId)
                        .setInstanceId(task.getInstanceId())
                        .setLevel(level)
                        .setMessage(message)
                        .setSequence(logSequence.incrementAndGet())
                        .setAssignmentToken(task.getAssignmentToken())
                        .build()
                )
                .build()
        );
    }

    private static String errorMessage(Throwable error) {
        if (error == null || error.getMessage() == null || error.getMessage().isBlank()) {
            return error == null ? "processor failed" : error.getClass().getName();
        }
        return error.getMessage();
    }

    private static String stackTrace(Throwable error) {
        StringWriter writer = new StringWriter();
        error.printStackTrace(new PrintWriter(writer));
        return writer.toString();
    }

    private static void printTaskLogLocally(String level, String message) {
        String line = "[tikeo-worker] " + message;
        if ("error".equalsIgnoreCase(level)) {
            System.err.println(line);
        } else {
            System.out.println(line);
        }
    }

    private final class ServerObserver
        implements StreamObserver<Worker.ServerMessage>
    {

        @Override
        public void onNext(Worker.ServerMessage message) {
            switch (message.getKindCase()) {
                case REGISTERED -> {
                    Worker.WorkerRegistered registered =
                        message.getRegistered();
                    workerId.set(registered.getWorkerId());
                    generation.set(registered.getGeneration());
                    fencingToken.set(registered.getFencingToken());
                    connected.set(true);
                    registrationLatch.countDown();
                }
                case DISPATCH_TASK -> handleDispatch(message.getDispatchTask());
                case PING, KIND_NOT_SET -> {
                    // Heartbeat acknowledgement; no-op for now.
                }
            }
        }

        @Override
        public void onError(Throwable error) {
            terminalError.compareAndSet(null, error);
            markDisconnected();
            registrationLatch.countDown();
            scheduleReconnect();
        }

        @Override
        public void onCompleted() {
            markDisconnected();
            registrationLatch.countDown();
            scheduleReconnect();
        }
    }
}
