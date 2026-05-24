package com.yhyzgn.tikee.core;

import tikee.worker.v1.Worker;
import tikee.worker.v1.WorkerTunnelServiceGrpc;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.stub.StreamObserver;
import java.net.URI;
import java.time.Duration;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Consumer;

/**
 * gRPC implementation of the active outbound Worker Tunnel client.
 */
public final class GrpcTikeeWorkerClient implements TikeeWorkerClient {
    private static final Duration DEFAULT_START_TIMEOUT = Duration.ofSeconds(10);

    private final WorkerRegistration registration;
    private final ManagedChannel channel;
    private final boolean ownsChannel;
    private final TaskProcessor processor;
    private final Duration heartbeatInterval;
    private final Duration startTimeout;
    private final Consumer<Worker.DispatchTask> dispatchObserver;
    private final ScheduledExecutorService tikee;
    private final ExecutorService processorExecutor;
    private final AtomicReference<String> workerId = new AtomicReference<>();
    private final AtomicReference<String> fencingToken = new AtomicReference<>("");
    private final AtomicReference<StreamObserver<Worker.WorkerMessage>> outbound = new AtomicReference<>();
    private final AtomicReference<Throwable> terminalError = new AtomicReference<>();
    private final AtomicLong generation = new AtomicLong(0);
    private final AtomicLong heartbeatSequence = new AtomicLong(0);
    private final AtomicLong logSequence = new AtomicLong(0);
    private volatile CountDownLatch registrationLatch = new CountDownLatch(1);
    private volatile ScheduledFuture<?> heartbeatTask;

    /**
     * Create a Worker Tunnel client from endpoint and registration metadata.
     *
     * @param endpoint tikee Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     */
    public GrpcTikeeWorkerClient(String endpoint, WorkerRegistration registration) {
        this(endpoint, registration, context -> TaskOutcome.succeeded());
    }

    /**
     * Create a Worker Tunnel client with a task processor.
     *
     * @param endpoint tikee Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     * @param processor task processor
     */
    public GrpcTikeeWorkerClient(String endpoint, WorkerRegistration registration, TaskProcessor processor) {
        this(endpoint, registration, processor, Duration.ofSeconds(10));
    }

    /**
     * Create a Worker Tunnel client with custom heartbeat interval.
     *
     * @param endpoint tikee Worker Tunnel endpoint, e.g. {@code http://127.0.0.1:9998}
     * @param registration worker registration metadata
     * @param processor task processor
     * @param heartbeatInterval heartbeat interval
     */
    public GrpcTikeeWorkerClient(
            String endpoint,
            WorkerRegistration registration,
            TaskProcessor processor,
            Duration heartbeatInterval) {
        this(
                channelForEndpoint(endpoint),
                true,
                registration,
                processor,
                heartbeatInterval,
                DEFAULT_START_TIMEOUT,
                ignored -> {});
    }

    GrpcTikeeWorkerClient(
            ManagedChannel channel,
            boolean ownsChannel,
            WorkerRegistration registration,
            TaskProcessor processor,
            Duration heartbeatInterval,
            Duration startTimeout,
            Consumer<Worker.DispatchTask> dispatchObserver) {
        this.registration = Objects.requireNonNull(registration, "registration");
        this.channel = Objects.requireNonNull(channel, "channel");
        this.ownsChannel = ownsChannel;
        this.processor = Objects.requireNonNull(processor, "processor");
        this.heartbeatInterval = positiveDuration(heartbeatInterval, "heartbeatInterval");
        this.startTimeout = positiveDuration(startTimeout, "startTimeout");
        this.dispatchObserver = Objects.requireNonNull(dispatchObserver, "dispatchObserver");
        ScheduledThreadPoolExecutor executor = new ScheduledThreadPoolExecutor(1);
        executor.setRemoveOnCancelPolicy(true);
        this.tikee = executor;
        this.processorExecutor = Executors.newCachedThreadPool(runnable -> {
            Thread thread = new Thread(runnable, "tikee-worker-java-processor");
            thread.setDaemon(true);
            return thread;
        });
    }

    @Override
    public synchronized void start() {
        if (outbound.get() != null) {
            return;
        }
        terminalError.set(null);
        registrationLatch = new CountDownLatch(1);
        WorkerTunnelServiceGrpc.WorkerTunnelServiceStub stub = WorkerTunnelServiceGrpc.newStub(channel);
        StreamObserver<Worker.WorkerMessage> requestObserver = stub.openTunnel(new ServerObserver());
        outbound.set(requestObserver);
        requestObserver.onNext(registerMessage());
        awaitRegistration();
        heartbeatTask = tikee.scheduleAtFixedRate(this::sendHeartbeatSafely,
                heartbeatInterval.toMillis(), heartbeatInterval.toMillis(), TimeUnit.MILLISECONDS);
    }

    @Override
    public String workerId() {
        return workerId.get();
    }

    @Override
    public void emitLog(String instanceId, String level, String message) {
        String assignedWorkerId = requireWorkerId();
        send(Worker.WorkerMessage.newBuilder()
                .setTaskLog(Worker.TaskLog.newBuilder()
                        .setWorkerId(assignedWorkerId)
                        .setInstanceId(Objects.requireNonNull(instanceId, "instanceId"))
                        .setLevel(Objects.requireNonNullElse(level, "info"))
                        .setMessage(Objects.requireNonNullElse(message, ""))
                        .setSequence(logSequence.incrementAndGet())
                        .build())
                .build());
    }

    @Override
    public synchronized void close() {
        Optional.ofNullable(heartbeatTask).ifPresent(task -> task.cancel(true));
        heartbeatTask = null;
        StreamObserver<Worker.WorkerMessage> observer = outbound.getAndSet(null);
        if (observer != null) {
            sendGracefulUnregister(observer);
            observer.onCompleted();
        }
        tikee.shutdownNow();
        processorExecutor.shutdownNow();
        if (ownsChannel) {
            channel.shutdownNow();
        }
    }

    private void sendGracefulUnregister(StreamObserver<Worker.WorkerMessage> observer) {
        String assignedWorkerId = workerId.get();
        if (assignedWorkerId == null || assignedWorkerId.isBlank()) {
            return;
        }
        observer.onNext(Worker.WorkerMessage.newBuilder()
                .setUnregister(Worker.UnregisterWorker.newBuilder()
                        .setWorkerId(assignedWorkerId)
                        .setGeneration(generation.get())
                        .setFencingToken(fencingToken.get())
                        .build())
                .build());
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
        String host = uri.getHost() == null ? uri.getSchemeSpecificPart() : uri.getHost();
        ManagedChannelBuilder<?> builder = ManagedChannelBuilder.forAddress(host, port);
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
        Worker.RegisterWorker.Builder builder = Worker.RegisterWorker.newBuilder()
                .setClientInstanceId(registration.clientInstanceId())
                .setNamespace(registration.namespace())
                .setApp(registration.app())
                .setCluster(registration.cluster())
                .setRegion(registration.region())
                .addAllCapabilities(registration.capabilities())
                .putAllLabels(registration.labels());
        return Worker.WorkerMessage.newBuilder().setRegister(builder).build();
    }

    private void awaitRegistration() {
        try {
            if (!registrationLatch.await(startTimeout.toMillis(), TimeUnit.MILLISECONDS)) {
                throw new WorkerClientException("worker registration timed out");
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new WorkerClientException("worker registration interrupted", error);
        }
        Throwable error = terminalError.get();
        if (error != null) {
            throw new WorkerClientException("worker tunnel failed during registration", error);
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

    private void sendHeartbeatSafely() {
        try {
            String assignedWorkerId = workerId.get();
            if (assignedWorkerId == null || assignedWorkerId.isBlank()) {
                return;
            }
            send(Worker.WorkerMessage.newBuilder()
                    .setHeartbeat(Worker.Heartbeat.newBuilder()
                            .setWorkerId(assignedWorkerId)
                            .setSequence(heartbeatSequence.incrementAndGet())
                            .setGeneration(generation.get())
                            .setFencingToken(fencingToken.get())
                            .build())
                    .build());
        } catch (RuntimeException error) {
            terminalError.compareAndSet(null, error);
        }
    }

    private void send(Worker.WorkerMessage message) {
        StreamObserver<Worker.WorkerMessage> observer = outbound.get();
        if (observer == null) {
            throw new WorkerClientException("worker tunnel is not open");
        }
        observer.onNext(message);
    }

    private void handleDispatch(Worker.DispatchTask task) {
        dispatchObserver.accept(task);
        processorExecutor.submit(() -> {
            TaskOutcome outcome;
            if (task.hasProcessorBinding() && task.getProcessorBinding().hasWasm()) {
                outcome = TaskOutcome.failed("wasm processor binding is not supported by Java SDK yet");
            } else if (task.hasProcessorBinding() && task.getProcessorBinding().hasScript()) {
                outcome = TaskOutcome.failed("script processor binding is not supported by Java SDK yet");
            } else {
                try {
                    outcome = processor.process(new TaskContext(
                            task.getJobId(),
                            task.getProcessorName(),
                            task.getInstanceId(),
                            task.getPayload().toByteArray()));
                } catch (Exception error) {
                    outcome = TaskOutcome.failed(error.getMessage());
                }
            }
            send(Worker.WorkerMessage.newBuilder()
                    .setTaskResult(Worker.TaskResult.newBuilder()
                            .setWorkerId(requireWorkerId())
                            .setInstanceId(task.getInstanceId())
                            .setSuccess(outcome.success())
                            .setMessage(outcome.message())
                            .build())
                    .build());
        });
    }

    private final class ServerObserver implements StreamObserver<Worker.ServerMessage> {
        @Override
        public void onNext(Worker.ServerMessage message) {
            switch (message.getKindCase()) {
                case REGISTERED -> {
                    Worker.WorkerRegistered registered = message.getRegistered();
                    workerId.set(registered.getWorkerId());
                    generation.set(registered.getGeneration());
                    fencingToken.set(registered.getFencingToken());
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
            registrationLatch.countDown();
        }

        @Override
        public void onCompleted() {
            registrationLatch.countDown();
        }
    }
}
