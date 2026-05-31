package com.yhyzgn.tikee.worker.client;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.yhyzgn.tikee.processor.ProcessorCapabilityProvider;
import com.yhyzgn.tikee.processor.TaskContext;
import com.yhyzgn.tikee.processor.TaskOutcome;
import com.yhyzgn.tikee.processor.TaskProcessor;
import com.yhyzgn.tikee.worker.WorkerRegistration;
import com.yhyzgn.tikee.wasm.WasmRunnerRegistry;
import com.yhyzgn.tikee.wasm.WasmRunnerTask;
import com.yhyzgn.tikee.script.ScriptRunner;
import com.yhyzgn.tikee.script.ScriptRunnerKind;
import com.yhyzgn.tikee.script.ScriptRunnerLogSink;
import com.yhyzgn.tikee.script.ScriptRunnerRegistry;
import com.yhyzgn.tikee.script.ScriptRunnerTask;

import tikee.worker.v1.Worker;
import tikee.worker.v1.WorkerTunnelServiceGrpc;
import io.grpc.ManagedChannel;
import io.grpc.Server;
import io.grpc.inprocess.InProcessChannelBuilder;
import io.grpc.inprocess.InProcessServerBuilder;
import io.grpc.stub.StreamObserver;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.time.Duration;
import java.util.HexFormat;
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
            TaskProcessor processor = new CapabilityAwareProcessor();
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    registration,
                    processor,
                    Duration.ofMillis(50),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitMessages(2);
            client.emitLog("instance-1", "info", "hello");
            service.awaitTaskLogs(1);

            assertEquals("assigned-java-worker", client.workerId());
            assertTrue(client.connected());
            client.close();
            service.awaitUnregister();
            Worker.RegisterWorker register = service.messages.get(0).getRegister();
            assertEquals("java-instance-1", register.getClientInstanceId());
            assertTrue(register.getCapabilitiesList().contains("java"));
            assertTrue(register.getElection().getEnabled());
            assertEquals("", register.getElection().getDomain());
            assertEquals(100, register.getElection().getPriority());
            assertTrue(register.getStructuredCapabilities().getSdkProcessorsList().stream()
                    .anyMatch(item -> "demo.echo".equals(item.getName())));
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

    private static final class CapabilityAwareProcessor implements TaskProcessor, ProcessorCapabilityProvider {
        @Override
        public TaskOutcome process(TaskContext context) {
            return TaskOutcome.succeeded();
        }

        @Override
        public List<String> capabilities() {
            return List.of("processor:demo.echo");
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
            service.awaitTaskLogs(2);
            assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("received task instance-1")
                            && log.getAssignmentToken().equals("java-assign-token")));
            assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("completed task instance-1 success=true")
                            && log.getAssignmentToken().equals("java-assign-token")));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void dispatchedTaskCapturesProcessorStdoutAsTaskLog() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-sql")
                .setProcessorName("billing.sql-sync")
                .setInstanceId("instance-sql")
                .setPayload(com.google.protobuf.ByteString.copyFromUtf8("{}"))
                .setAssignmentToken("java-assign-token")
                .build());
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
                    new WorkerRegistration("java-instance-stdout", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        System.out.println("[billing.sql-sync] plugin SQL processor received payload='"
                                + new String(context.payload(), StandardCharsets.UTF_8) + "'");
                        return new TaskOutcome(true, "sql-plugin-ok");
                    },
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            PrintStream originalOut = System.out;
            ByteArrayOutputStream console = new ByteArrayOutputStream();
            try (PrintStream capture = new PrintStream(console, true, StandardCharsets.UTF_8)) {
                System.setOut(capture);
                client.start();
                service.awaitResult();
                service.awaitTaskLogs(3);
                client.close();
            } finally {
                System.setOut(originalOut);
            }

            String processorLog = "[billing.sql-sync] plugin SQL processor received payload='{}'";
            assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains(processorLog)
                            && log.getAssignmentToken().equals("java-assign-token")));
            assertEquals(1, countOccurrences(console.toString(StandardCharsets.UTF_8), processorLog));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void serverCompletionEventuallyReconnectsWithoutForgettingWorkerId() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(null, true);
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
                    new WorkerRegistration("java-instance-disconnect", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> TaskOutcome.succeeded(),
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitServerCompleted();
            service.awaitRegisterCount(2);

            assertEquals("assigned-java-worker", client.workerId());
            assertTrue(client.connected());
            client.close();
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void serverCompletionTriggersReconnectWithoutChangingWorkerId() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(null, true);
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
                    new WorkerRegistration("java-instance-reconnect", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> TaskOutcome.succeeded(),
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    Duration.ofMillis(10),
                    Duration.ofMillis(20),
                    ignored -> {});

            client.start();
            service.awaitServerCompleted();
            service.awaitRegisterCount(2);

            assertEquals("assigned-java-worker", client.workerId());
            assertTrue(client.connected());
            client.close();
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void directClientRegistrationIncludesWasmRunnerCapability() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService();
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            WasmRunnerRegistry wasmRunners = new WasmRunnerRegistry()
                    .register((task, logSink) -> TaskOutcome.succeeded());
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration(
                            "java-instance-wasm-cap", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> TaskOutcome.succeeded(),
                    new ScriptRunnerRegistry(),
                    wasmRunners,
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    Duration.ofMillis(10),
                    Duration.ofMillis(20),
                    ignored -> {});

            client.start();
            service.awaitRegisterCount(1);
            client.close();

            Worker.RegisterWorker register = service.messages.get(0).getRegister();
            assertTrue(register.getStructuredCapabilities().getScriptRunnersList().stream()
                    .anyMatch(runner -> "wasm".equals(runner.getLanguage())));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchReportsUnregisteredRunnerWithoutInvokingProcessor() throws Exception {
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

            assertNull(observed.get(), "unregistered wasm binding must not invoke the Java processor");
            Worker.TaskResult result = service.result.get();
            assertEquals("assigned-java-worker", result.getWorkerId());
            assertEquals("instance-wasm", result.getInstanceId());
            assertTrue(!result.getSuccess());
            assertTrue(result.getMessage().contains("wasm runner is not registered"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchUsesRegisteredSandboxRunner() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        byte[] module = "wasm-module".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-wasm")
                .setProcessorName("script:wasm")
                .setInstanceId("instance-wasm")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setWasm(Worker.WasmProcessorBinding.newBuilder()
                                .setScriptId("script_wasm")
                                .setVersion("1.0.0")
                                .setModule(com.google.protobuf.ByteString.copyFrom(module))
                                .setRuntime("wasmtime")
                                .setEntrypoint("_start")
                                .setTimeoutMs(1000)
                                .setMaxMemoryBytes(1048576)
                                .setFuel(1000000)
                                .setVersionId("sv_wasm")
                                .setVersionNumber(1)
                                .setModuleSha256(sha256(module))
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
            AtomicReference<TaskContext> observedProcessor = new AtomicReference<>();
            AtomicReference<WasmRunnerTask> observedWasm = new AtomicReference<>();
            WasmRunnerRegistry registry = new WasmRunnerRegistry().register((task, logSink) -> {
                observedWasm.set(task);
                logSink.log("info", "[wasm] hello from sandbox");
                return new TaskOutcome(true, "wasm ok");
            });
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-wasm", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        observedProcessor.set(context);
                        return TaskOutcome.succeeded();
                    },
                    new ScriptRunnerRegistry(),
                    registry,
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    Duration.ofMillis(10),
                    Duration.ofMillis(20),
                    ignored -> {});

            client.start();
            service.awaitResult();
            service.awaitTaskLogs(3);
            client.close();

            assertNull(observedProcessor.get(), "wasm binding must not invoke SDK processor handlers");
            assertEquals("script_wasm", observedWasm.get().scriptId());
            Worker.TaskResult result = service.result.get();
            assertTrue(result.getSuccess());
            assertEquals("wasm ok", result.getMessage());
            assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("[wasm] hello from sandbox")));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void scriptBoundDispatchReportsUnregisteredRunnerWithoutInvokingProcessor() throws Exception {
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
                                .setContentSha256(sha256("exit 0"))
                                .setTimeoutMs(1000)
                                .setMaxMemoryBytes(1048576)
                                .setMaxOutputBytes(1048576)
                                .setAllowNetwork(true)
                                .addAllowedNetworkHosts("api.example.com")
                                .addReadOnlyPaths("/data/input")
                                .addWritablePaths("/data/output")
                                .addSecretRefs("secret:db-readonly")
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
            assertTrue(result.getMessage().contains("script runner is not registered"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }


    @Test
    void scriptBoundDispatchUsesRegisteredSandboxRunner() throws Exception {
        String serverName = "tikee-worker-test-" + UUID.randomUUID();
        String script = "echo hello";
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-script")
                .setProcessorName("script:shell")
                .setInstanceId("instance-script")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setScript(Worker.ScriptProcessorBinding.newBuilder()
                                .setScriptId("script_shell")
                                .setVersion("1.0.0")
                                .setLanguage("shell")
                                .setContent(com.google.protobuf.ByteString.copyFromUtf8(script))
                                .setVersionId("sv_1")
                                .setVersionNumber(1)
                                .setContentSha256(sha256(script))
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
            AtomicReference<TaskContext> observedProcessor = new AtomicReference<>();
            AtomicReference<ScriptRunnerTask> observedScript = new AtomicReference<>();
            ScriptRunnerRegistry registry = new ScriptRunnerRegistry().register(new ScriptRunner() {
                @Override
                public ScriptRunnerKind kind() {
                    return ScriptRunnerKind.SHELL;
                }

                @Override
                public TaskOutcome run(ScriptRunnerTask task) {
                    observedScript.set(task);
                    return new TaskOutcome(true, "script ok");
                }

                @Override
                public TaskOutcome run(ScriptRunnerTask task, ScriptRunnerLogSink logSink) {
                    observedScript.set(task);
                    logSink.log("info", "[script] hello from sandbox");
                    return new TaskOutcome(true, "script ok");
                }
            });
            GrpcTikeeWorkerClient client = new GrpcTikeeWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-script", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        observedProcessor.set(context);
                        return TaskOutcome.succeeded();
                    },
                    registry,
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    Duration.ofMillis(10),
                    Duration.ofMillis(20),
                    ignored -> {});

            PrintStream originalOut = System.out;
            ByteArrayOutputStream console = new ByteArrayOutputStream();
            try (PrintStream capture = new PrintStream(console, true, StandardCharsets.UTF_8)) {
                System.setOut(capture);
                client.start();
                service.awaitResult();
                service.awaitTaskLogs(3);
                client.close();
            } finally {
                System.setOut(originalOut);
            }

            assertNull(observedProcessor.get(), "script binding must not invoke SDK processor handlers");
            assertEquals("script_shell", observedScript.get().scriptId());
            assertEquals("shell", observedScript.get().language());
            Worker.TaskResult result = service.result.get();
            assertTrue(result.getSuccess());
            assertEquals("script ok", result.getMessage());
            String scriptLog = "[script] hello from sandbox";
            assertEquals(1, service.taskLogs.stream()
                    .filter(log -> log.getMessage().contains(scriptLog))
                    .count());
            assertTrue(console.toString(StandardCharsets.UTF_8).contains("[tikee-worker] " + scriptLog));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    private static int countOccurrences(String value, String needle) {
        int count = 0;
        int index = 0;
        while ((index = value.indexOf(needle, index)) >= 0) {
            count += 1;
            index += needle.length();
        }
        return count;
    }

    private static String sha256(String content) throws Exception {
        return sha256(content.getBytes(StandardCharsets.UTF_8));
    }

    private static String sha256(byte[] content) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(content));
    }

    private static final class RecordingTunnelService extends WorkerTunnelServiceGrpc.WorkerTunnelServiceImplBase {
        private final List<Worker.WorkerMessage> messages = new ArrayList<>();
        private final Worker.DispatchTask dispatchTask;
        private final boolean completeAfterRegister;
        private final CountDownLatch messageLatch = new CountDownLatch(3);
        private final CountDownLatch resultLatch = new CountDownLatch(1);
        private final CountDownLatch unregisterLatch = new CountDownLatch(1);
        private final CountDownLatch serverCompletedLatch = new CountDownLatch(1);
        private final List<Worker.TaskLog> taskLogs = new ArrayList<>();
        private final AtomicReference<Worker.TaskResult> result = new AtomicReference<>();
        private final AtomicReference<Worker.UnregisterWorker> unregister = new AtomicReference<>();
        private int registerCount;

        private RecordingTunnelService() {
            this(null);
        }

        private RecordingTunnelService(Worker.DispatchTask dispatchTask) {
            this(dispatchTask, false);
        }

        private RecordingTunnelService(Worker.DispatchTask dispatchTask, boolean completeAfterRegister) {
            this.dispatchTask = dispatchTask;
            this.completeAfterRegister = completeAfterRegister;
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
                        synchronized (messages) {
                            registerCount += 1;
                        }
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
                        if (completeAfterRegister && registerCount == 1) {
                            responseObserver.onCompleted();
                            serverCompletedLatch.countDown();
                        }
                    }
                    if (message.hasHeartbeat()) {
                        responseObserver.onNext(Worker.ServerMessage.newBuilder()
                                .setPing(Worker.Ping.newBuilder().setSequence(message.getHeartbeat().getSequence()).build())
                                .build());
                    }
                    if (message.hasTaskLog()) {
                        synchronized (taskLogs) {
                            taskLogs.add(message.getTaskLog());
                        }
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

        private void awaitTaskLogs(int count) throws InterruptedException {
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2);
            while (System.nanoTime() < deadline) {
                synchronized (taskLogs) {
                    if (taskLogs.size() >= count) {
                        return;
                    }
                }
                TimeUnit.MILLISECONDS.sleep(50);
            }
            synchronized (taskLogs) {
                assertTrue(taskLogs.size() >= count, "expected at least " + count + " task logs but got " + taskLogs.size());
            }
        }

        private void awaitUnregister() throws InterruptedException {
            assertTrue(unregisterLatch.await(2, TimeUnit.SECONDS));
        }

        private void awaitServerCompleted() throws InterruptedException {
            assertTrue(serverCompletedLatch.await(2, TimeUnit.SECONDS));
        }

        private void awaitRegisterCount(int count) throws InterruptedException {
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2);
            while (System.nanoTime() < deadline) {
                synchronized (messages) {
                    if (registerCount >= count) {
                        return;
                    }
                }
                TimeUnit.MILLISECONDS.sleep(20);
            }
            synchronized (messages) {
                assertTrue(registerCount >= count, "expected at least " + count + " registrations but got " + registerCount);
            }
        }
    }
}
