package cn.recycloud.scheduler.sdk.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import scheduler.worker.v1.Worker;
import scheduler.worker.v1.WorkerTunnelServiceGrpc;
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

class GrpcSchedulerWorkerClientTest {
    @Test
    void startRegistersWithClientInstanceIdAndUsesAssignedWorkerId() throws Exception {
        String serverName = "scheduler-worker-test-" + UUID.randomUUID();
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
            GrpcSchedulerWorkerClient client = new GrpcSchedulerWorkerClient(
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
                    .anyMatch(message -> "assigned-java-worker".equals(message.getHeartbeat().getWorkerId())));
            assertTrue(service.messages.stream()
                    .filter(Worker.WorkerMessage::hasTaskLog)
                    .anyMatch(message -> "assigned-java-worker".equals(message.getTaskLog().getWorkerId())));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void dispatchedTaskReportsProcessorResultWithAssignedWorkerId() throws Exception {
        String serverName = "scheduler-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-1")
                .setProcessorName("demo.echo")
                .setInstanceId("instance-1")
                .setPayload(com.google.protobuf.ByteString.copyFromUtf8("hello"))
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            AtomicReference<TaskContext> observed = new AtomicReference<>();
            GrpcSchedulerWorkerClient client = new GrpcSchedulerWorkerClient(
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
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchReportsUnsupportedWithoutInvokingProcessor() throws Exception {
        String serverName = "scheduler-worker-test-" + UUID.randomUUID();
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
            GrpcSchedulerWorkerClient client = new GrpcSchedulerWorkerClient(
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

    private static final class RecordingTunnelService extends WorkerTunnelServiceGrpc.WorkerTunnelServiceImplBase {
        private final List<Worker.WorkerMessage> messages = new ArrayList<>();
        private final Worker.DispatchTask dispatchTask;
        private final CountDownLatch messageLatch = new CountDownLatch(3);
        private final CountDownLatch resultLatch = new CountDownLatch(1);
        private final AtomicReference<Worker.TaskResult> result = new AtomicReference<>();

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
                assertTrue(messageLatch.await(100, TimeUnit.MILLISECONDS));
            }
            synchronized (messages) {
                assertTrue(messages.size() >= count, "expected at least " + count + " messages but got " + messages.size());
            }
        }

        private void awaitResult() throws InterruptedException {
            assertTrue(resultLatch.await(2, TimeUnit.SECONDS));
        }
    }
}
