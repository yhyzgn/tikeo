package com.yhyzgn.tikee.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import tikee.worker.v1.Worker;
import tikee.worker.v1.WorkerTunnelServiceGrpc;
import io.grpc.ManagedChannel;
import io.grpc.Server;
import io.grpc.inprocess.InProcessChannelBuilder;
import io.grpc.inprocess.InProcessServerBuilder;
import io.grpc.stub.StreamObserver;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;

class GrpcTikeeWorkerClientTest {
    @Test
    void startRegistersWithClientInstanceIdAndUsesAssignedWorkerId() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService();
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            WorkerRegistration registration = new WorkerRegistration(
                    "java-instance-1",
                    "default",
                    "billing",
                    "local",
                    "local",
                    List.of("java"),
                    Map.of("runtime", "java"));
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    registration,
                    context -> TaskOutcome.succeeded(),
                    Duration.ofMillis(50),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitMessages(2);
            client.emitLog("instance-1", "info", "hello");
            service.awaitMessages(3);
            client.close();

            assertEquals("assigned-java-worker", client.workerId());
            Worker.RegisterWorker register = service.messages.get(0).getRegister();
            assertEquals("java-instance-1", register.getClientInstanceId());
            assertTrue(service.messages.stream()
                    .filter(Worker.WorkerMessage::hasHeartbeat)
                    .anyMatch(message -> "assigned-java-worker".equals(message.getHeartbeat().getWorkerId())
                            && message.getHeartbeat().getGeneration() == 1
                            && "java-fencing-token".equals(message.getHeartbeat().getFencingToken())));
            assertTrue(service.messages.stream()
                    .filter(Worker.WorkerMessage::hasTaskLog)
                    .anyMatch(message -> "assigned-java-worker".equals(message.getTaskLog().getWorkerId())));
            assertTrue(service.messages.stream()
                    .filter(Worker.WorkerMessage::hasUnregister)
                    .anyMatch(message -> "assigned-java-worker".equals(message.getUnregister().getWorkerId())
                            && message.getUnregister().getGeneration() == 1
                            && "java-fencing-token".equals(message.getUnregister().getFencingToken())));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void closeSendsGracefulUnregisterWithGenerationAndFencingToken() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService();
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-stop", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> TaskOutcome.succeeded(),
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            client.close();
            service.awaitUnregister();

            Worker.UnregisterWorker unregister = service.unregister.get();
            assertEquals("assigned-java-worker", unregister.getWorkerId());
            assertEquals(1, unregister.getGeneration());
            assertEquals("java-fencing-token", unregister.getFencingToken());
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void dispatchedTaskReportsProcessorResultWithAssignedWorkerId() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-1")
                .setProcessorName("demo.echo")
                .setInstanceId("instance-1")
                .setPayload(com.google.protobuf.ByteString.copyFromUtf8("hello"))
                .setAssignmentToken("java-assign-token")
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            AtomicReference<TaskContext> observed = new AtomicReference<>();
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-2", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        observed.set(context);
                        return TaskOutcome.succeeded();
                    },
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitResult();
            client.close();

            assertEquals("instance-1", observed.get().instanceId());
            assertEquals("demo.echo", observed.get().processorName());
            Worker.TaskResult result = service.result.get();
            assertTrue(result.getSuccess());
            assertEquals("assigned-java-worker", result.getWorkerId());
            assertEquals("java-assign-token", result.getAssignmentToken());
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchReportsUnsupportedWithoutInvokingProcessor() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-wasm")
                .setProcessorName("script:script_wasm")
                .setInstanceId("instance-wasm")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setWasm(Worker.WasmProcessorBinding.newBuilder()
                                .setScriptId("script_wasm")
                                .setVersion("1.0.0")
                                .setRuntime("wasmtime")
                                .setEntrypoint("_start")
                                .setTimeoutMs(1000)
                                .setMaxMemoryBytes(1048576)
                                .setFuel(1000000)
                                .build())
                        .build())
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            AtomicReference<TaskContext> observed = new AtomicReference<>();
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-wasm", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        observed.set(context);
                        return TaskOutcome.succeeded();
                    },
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitResult();
            client.close();

            assertNull(observed.get(), "unsupported wasm binding must not invoke the Java processor");
            Worker.TaskResult result = service.result.get();
            assertEquals("assigned-java-worker", result.getWorkerId());
            assertEquals("instance-wasm", result.getInstanceId());
            assertTrue(!result.getSuccess());
            assertTrue(result.getMessage().contains("wasm"));
            assertTrue(result.getMessage().contains("not supported"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void scriptBoundDispatchReportsUnsupportedWithoutInvokingProcessor() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-script")
                .setProcessorName("script:script_shell")
                .setInstanceId("instance-script")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setScript(Worker.ScriptProcessorBinding.newBuilder()
                                .setScriptId("script_shell")
                                .setVersion("1.0.0")
                                .setLanguage("shell")
                                .setContent(com.google.protobuf.ByteString.copyFromUtf8("exit 0"))
                                .setVersionId("sv_1")
                                .setVersionNumber(1)
                                .setContentSha256("digest")
                                .setTimeoutMs(1000)
                                .setMaxMemoryBytes(1048576)
                                .setMaxOutputBytes(1048576)
                                .build())
                        .build())
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            AtomicReference<TaskContext> observed = new AtomicReference<>();
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-script", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        observed.set(context);
                        return TaskOutcome.succeeded();
                    },
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitResult();
            client.close();

            assertNull(observed.get(), "unsupported script binding must not invoke the Java processor");
            Worker.TaskResult result = service.result.get();
            assertEquals("assigned-java-worker", result.getWorkerId());
            assertEquals("instance-script", result.getInstanceId());
            assertTrue(!result.getSuccess());
            assertTrue(result.getMessage().contains("script"));
            assertTrue(result.getMessage().contains("not supported"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    private static final class RecordingTunnelService extends WorkerTunnelServiceGrpc.WorkerTunnelServiceImplBase {
        private final List<Worker.WorkerMessage> messages = new ArrayList<>();
        private final Worker.DispatchTask dispatchTask;
        private final CountDownLatch messageLatch = new CountDownLatch(3);
        private final CountDownLatch resultLatch = new CountDownLatch(1);
        private final CountDownLatch unregisterLatch = new CountDownLatch(1);
        private final AtomicReference<Worker.TaskResult> result = new AtomicReference<>();
        private final AtomicReference<Worker.UnregisterWorker> unregister = new AtomicReference<>();

        private RecordingTunnelService() {
            this(null);
        }

        private RecordingTunnelService(Worker.DispatchTask dispatchTask) {
            this.dispatchTask = dispatchTask;
        }

        @Override
        public StreamObserver<Worker.WorkerMessage> openTunnel(StreamObserver<Worker.ServerMessage> responseObserver) {
            return new StreamObserver<>() {
                @Override
                public void onNext(Worker.WorkerMessage message) {
                    synchronized (messages) {
                        messages.add(message);
                    }
                    messageLatch.countDown();
                    if (message.hasRegister()) {
                        responseObserver.onNext(Worker.ServerMessage.newBuilder()
                                .setRegistered(Worker.WorkerRegistered.newBuilder()
                                        .setWorkerId("assigned-java-worker")
                                        .setLeaseSeconds(30)
                                        .setGeneration(1)
                                        .setFencingToken("java-fencing-token")
                                        .build())
                                .build());
                        if (dispatchTask != null) {
                            responseObserver.onNext(Worker.ServerMessage.newBuilder()
                                    .setDispatchTask(dispatchTask)
                                    .build());
                        }
                    }
                    if (message.hasHeartbeat()) {
                        responseObserver.onNext(Worker.ServerMessage.newBuilder()
                                .setPing(Worker.Ping.newBuilder().setSequence(message.getHeartbeat().getSequence()).build())
                                .build());
                    }
                    if (message.hasTaskResult()) {
                        result.set(message.getTaskResult());
                        resultLatch.countDown();
                    }
                    if (message.hasUnregister()) {
                        unregister.set(message.getUnregister());
                        unregisterLatch.countDown();
                    }
                }

                @Override
                public void onError(Throwable throwable) {
                    responseObserver.onError(throwable);
                }

                @Override
                public void onCompleted() {
                    responseObserver.onCompleted();
                }
            };
        }

        private void awaitMessages(int count) throws InterruptedException {
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2);
            while (System.nanoTime() < deadline) {
                synchronized (messages) {
                    if (messages.size() >= count) {
                        return;
                    }
                }
                assertTrue(messageLatch.await(500, TimeUnit.MILLISECONDS));
            }
            synchronized (messages) {
                assertTrue(messages.size() >= count, "expected at least " + count + " messages but got " + messages.size());
            }
        }

        private void awaitResult() throws InterruptedException {
            assertTrue(resultLatch.await(2, TimeUnit.SECONDS));
        }

        private void awaitUnregister() throws InterruptedException {
            assertTrue(unregisterLatch.await(2, TimeUnit.SECONDS));
        }
    }
}
