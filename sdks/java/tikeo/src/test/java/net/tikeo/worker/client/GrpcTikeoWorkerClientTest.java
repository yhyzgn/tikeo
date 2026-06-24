package net.tikeo.worker.client;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.Logger;
import com.google.protobuf.ByteString;
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
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import net.tikeo.logging.TikeoTaskLogbackAppender;
import net.tikeo.processor.TaskContext;
import net.tikeo.processor.TaskOutcome;
import net.tikeo.processor.TaskProcessor;
import net.tikeo.script.ScriptRunner;
import net.tikeo.script.ScriptRunnerKind;
import net.tikeo.script.ScriptRunnerLogSink;
import net.tikeo.script.ScriptRunnerRegistry;
import net.tikeo.script.ScriptRunnerTask;
import net.tikeo.wasm.WasmRunnerRegistry;
import net.tikeo.wasm.WasmRunnerTask;
import net.tikeo.worker.WorkerCapabilityProvider;
import net.tikeo.worker.WorkerCapabilitySet;
import net.tikeo.worker.WorkerRegistration;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.slf4j.LoggerFactory;
import tikeo.worker.v1.Worker;
import tikeo.worker.v1.WorkerTunnelServiceGrpc;

class GrpcTikeoWorkerClientTest {
    @Test
    void startRegistersWithClientInstanceIdAndUsesAssignedWorkerId() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertEquals("assigned-java-worker", client.workerId());
            awaitConnected(client);
            client.close();
            service.awaitUnregister();
            Worker.RegisterWorker register = service.messages.get(0).getRegister();
            Assertions.assertEquals("java-instance-1", register.getClientInstanceId());
            Assertions.assertTrue(register.getCapabilitiesList().contains("java"));
            Assertions.assertTrue(register.getElection().getEnabled());
            Assertions.assertEquals("", register.getElection().getDomain());
            Assertions.assertEquals(100, register.getElection().getPriority());
            Assertions.assertTrue(register.getStructuredCapabilities().getNormalProcessorsList().stream()
                    .anyMatch(item -> "demo.echo".equals(item.getName())));
            Assertions.assertTrue(service.hasHeartbeat("assigned-java-worker", 1, "java-fencing-token"));
            Assertions.assertTrue(service.hasTaskLogFrom("assigned-java-worker"));
            Assertions.assertTrue(service.hasUnregister("assigned-java-worker", 1, "java-fencing-token"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    private static final class CapabilityAwareProcessor implements TaskProcessor, WorkerCapabilityProvider {
        @Override
        public TaskOutcome process(TaskContext context) {
            return TaskOutcome.succeeded();
        }

        @Override
        public WorkerCapabilitySet workerCapabilities() {
            return new WorkerCapabilitySet(
                    List.of(),
                    List.of(new WorkerCapabilitySet.Processor("demo.echo", "Echo processor")),
                    List.of(),
                    List.of());
        }
    }


    @Test
    void startDoesNotFailApplicationWhenTikeoServerIsUnavailable() {
        ManagedChannel channel = InProcessChannelBuilder.forName("missing-tikeo-server-" + UUID.randomUUID()).directExecutor().build();
        try {
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-offline", "default", "billing", "local", "local", List.of("java"), Map.of()),
                    context -> TaskOutcome.succeeded(),
                    Duration.ofMillis(50),
                    Duration.ofMillis(50),
                    Duration.ofMillis(10),
                    Duration.ofMillis(20),
                    ignored -> {});

            Assertions.assertDoesNotThrow(client::start);
            Assertions.assertFalse(client.connected());
            client.close();
        } finally {
            channel.shutdownNow();
        }
    }

    @Test
    void closeSendsGracefulUnregisterWithGenerationAndFencingToken() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService();
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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
            Assertions.assertEquals("assigned-java-worker", unregister.getWorkerId());
            Assertions.assertEquals(1, unregister.getGeneration());
            Assertions.assertEquals("java-fencing-token", unregister.getFencingToken());
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void dispatchedTaskReportsProcessorResultWithAssignedWorkerId() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-1")
                .setProcessorName("demo.echo")
                .setInstanceId("instance-1")
                .setPayload(ByteString.copyFromUtf8("hello"))
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertEquals("instance-1", observed.get().instanceId());
            Assertions.assertEquals("demo.echo", observed.get().processorName());
            Worker.TaskResult result = service.result.get();
            Assertions.assertTrue(result.getSuccess());
            Assertions.assertEquals("assigned-java-worker", result.getWorkerId());
            Assertions.assertEquals("java-assign-token", result.getAssignmentToken());
            service.awaitTaskLogs(2);
            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("received task instance-1")
                            && log.getAssignmentToken().equals("java-assign-token")));
            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("completed task instance-1 success=true")
                            && log.getAssignmentToken().equals("java-assign-token")));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void dispatchedTaskRecordsOnlyTaskLoggerLines() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-sql")
                .setProcessorName("billing.sql-sync")
                .setInstanceId("instance-sql")
                .setPayload(ByteString.copyFromUtf8("{}"))
                .setAssignmentToken("java-assign-token")
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-stdout", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        System.out.println("[billing.sql-sync] stdout should stay console-only payload='"
                                + new String(context.payload(), StandardCharsets.UTF_8) + "'");
                        context.logInfo("[billing.sql-sync] plugin SQL processor received payload='"
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
            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains(processorLog)
                            && log.getAssignmentToken().equals("java-assign-token")));
            Assertions.assertTrue(service.taskLogs.stream()
                    .noneMatch(log -> log.getMessage().contains("stdout should stay console-only")));
            Assertions.assertTrue(console.toString(StandardCharsets.UTF_8).contains("stdout should stay console-only"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }


    @Test
    void dispatchedTaskCapturesSlf4jLogsInsideTaskScopeOnly() throws Exception {
        String loggerName = "net.tikeo.worker.client.bridge-scope-test-" + UUID.randomUUID();
        Logger scopedLogger = (Logger) LoggerFactory.getLogger(loggerName);
        TikeoTaskLogbackAppender appender = new TikeoTaskLogbackAppender();
        appender.setContext(scopedLogger.getLoggerContext());
        appender.start();
        scopedLogger.addAppender(appender);
        scopedLogger.setLevel(Level.INFO);
        scopedLogger.setAdditive(false);

        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-logback")
                .setProcessorName("demo.logback")
                .setInstanceId("instance-logback")
                .setPayload(ByteString.copyFromUtf8("{}"))
                .setAssignmentToken("java-assign-token")
                .build());
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            scopedLogger.info("outside task should not be captured");
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
                    channel,
                    false,
                    new WorkerRegistration("java-instance-logback", "default", "billing", "local", "local", List.of(), Map.of()),
                    context -> {
                        scopedLogger.info("ordinary info payload={}", new String(context.payload(), StandardCharsets.UTF_8));
                        scopedLogger.error("ordinary error instance={}", context.instanceId());
                        return new TaskOutcome(true, "logback-ok");
                    },
                    Duration.ofSeconds(60),
                    Duration.ofSeconds(2),
                    ignored -> {});

            client.start();
            service.awaitResult();
            service.awaitTaskLogs(4);
            client.close();
            scopedLogger.info("after task should not be captured");

            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getLevel().equals("info")
                            && log.getMessage().contains("ordinary info payload={}")
                            && log.getAssignmentToken().equals("java-assign-token")));
            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getLevel().equals("error")
                            && log.getMessage().contains("ordinary error instance=instance-logback")
                            && log.getAssignmentToken().equals("java-assign-token")));
            Assertions.assertTrue(service.taskLogs.stream()
                    .noneMatch(log -> log.getMessage().contains("outside task should not be captured")
                            || log.getMessage().contains("after task should not be captured")));
        } finally {
            scopedLogger.detachAppender(appender);
            appender.stop();
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void serverCompletionEventuallyReconnectsWithoutForgettingWorkerId() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(null, true);
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertEquals("assigned-java-worker", client.workerId());
            awaitConnected(client);
            client.close();
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void serverCompletionTriggersReconnectWithoutChangingWorkerId() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(null, true);
        Server server = InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(service)
                .build()
                .start();
        ManagedChannel channel = InProcessChannelBuilder.forName(serverName).directExecutor().build();
        try {
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertEquals("assigned-java-worker", client.workerId());
            Assertions.assertTrue(client.connected());
            client.close();
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void directClientRegistrationIncludesWasmRunnerCapability() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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
            Assertions.assertTrue(register.getStructuredCapabilities().getScriptRunnersList().stream()
                    .anyMatch(runner -> "wasm".equals(runner.getLanguage())));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchReportsUnregisteredRunnerWithoutInvokingProcessor() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertNull(observed.get(), "unregistered wasm binding must not invoke the Java processor");
            Worker.TaskResult result = service.result.get();
            Assertions.assertEquals("assigned-java-worker", result.getWorkerId());
            Assertions.assertEquals("instance-wasm", result.getInstanceId());
            Assertions.assertTrue(!result.getSuccess());
            Assertions.assertTrue(result.getMessage().contains("wasm runner is not registered"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void wasmBoundDispatchUsesRegisteredSandboxRunner() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        byte[] module = "wasm-module".getBytes(StandardCharsets.UTF_8);
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-wasm")
                .setProcessorName("script:wasm")
                .setInstanceId("instance-wasm")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setWasm(Worker.WasmProcessorBinding.newBuilder()
                                .setScriptId("script_wasm")
                                .setVersion("1.0.0")
                                .setModule(ByteString.copyFrom(module))
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertNull(observedProcessor.get(), "wasm binding must not invoke normal processor handlers");
            Assertions.assertEquals("script_wasm", observedWasm.get().scriptId());
            Worker.TaskResult result = service.result.get();
            Assertions.assertTrue(result.getSuccess());
            Assertions.assertEquals("wasm ok", result.getMessage());
            Assertions.assertTrue(service.taskLogs.stream()
                    .anyMatch(log -> log.getMessage().contains("[wasm] hello from sandbox")));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }

    @Test
    void scriptBoundDispatchReportsUnregisteredRunnerWithoutInvokingProcessor() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
        RecordingTunnelService service = new RecordingTunnelService(Worker.DispatchTask.newBuilder()
                .setJobId("job-script")
                .setProcessorName("script:script_shell")
                .setInstanceId("instance-script")
                .setProcessorBinding(Worker.TaskProcessorBinding.newBuilder()
                        .setScript(Worker.ScriptProcessorBinding.newBuilder()
                                .setScriptId("script_shell")
                                .setVersion("1.0.0")
                                .setLanguage("shell")
                                .setContent(ByteString.copyFromUtf8("exit 0"))
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertNull(observed.get(), "unsupported script binding must not invoke the Java processor");
            Worker.TaskResult result = service.result.get();
            Assertions.assertEquals("assigned-java-worker", result.getWorkerId());
            Assertions.assertEquals("instance-script", result.getInstanceId());
            Assertions.assertTrue(!result.getSuccess());
            Assertions.assertTrue(result.getMessage().contains("script runner is not registered"));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }


    @Test
    void scriptBoundDispatchUsesRegisteredSandboxRunner() throws Exception {
        String serverName = "tikeo-worker-test-" + UUID.randomUUID();
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
                                .setContent(ByteString.copyFromUtf8(script))
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
            GrpcTikeoWorkerClient client = new GrpcTikeoWorkerClient(
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

            Assertions.assertNull(observedProcessor.get(), "script binding must not invoke normal processor handlers");
            Assertions.assertEquals("script_shell", observedScript.get().scriptId());
            Assertions.assertEquals("shell", observedScript.get().language());
            Worker.TaskResult result = service.result.get();
            Assertions.assertTrue(result.getSuccess());
            Assertions.assertEquals("script ok", result.getMessage());
            String scriptLog = "[script] hello from sandbox";
            Assertions.assertEquals(1, service.taskLogs.stream()
                    .filter(log -> log.getMessage().contains(scriptLog))
                    .count());
            Assertions.assertTrue(console.toString(StandardCharsets.UTF_8).contains("[tikeo-worker] " + scriptLog));
        } finally {
            channel.shutdownNow();
            server.shutdownNow();
        }
    }


    private static void awaitConnected(GrpcTikeoWorkerClient client) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
        while (System.nanoTime() < deadline) {
            if (client.connected()) {
                return;
            }
            TimeUnit.MILLISECONDS.sleep(20);
        }
        Assertions.assertTrue(client.connected(), "expected client to become connected after reconnect registration");
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
                Assertions.assertTrue(messageLatch.await(500, TimeUnit.MILLISECONDS));
            }
            synchronized (messages) {
                Assertions.assertTrue(messages.size() >= count, "expected at least " + count + " messages but got " + messages.size());
            }
        }

        private void awaitResult() throws InterruptedException {
            Assertions.assertTrue(resultLatch.await(2, TimeUnit.SECONDS));
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
                Assertions.assertTrue(taskLogs.size() >= count, "expected at least " + count + " task logs but got " + taskLogs.size());
            }
        }

        private void awaitUnregister() throws InterruptedException {
            Assertions.assertTrue(unregisterLatch.await(2, TimeUnit.SECONDS));
        }

        private void awaitServerCompleted() throws InterruptedException {
            Assertions.assertTrue(serverCompletedLatch.await(2, TimeUnit.SECONDS));
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
                Assertions.assertTrue(registerCount >= count, "expected at least " + count + " registrations but got " + registerCount);
            }
        }

        private boolean hasHeartbeat(String workerId, long generation, String fencingToken) {
            synchronized (messages) {
                return messages.stream()
                        .filter(Worker.WorkerMessage::hasHeartbeat)
                        .anyMatch(message -> workerId.equals(message.getHeartbeat().getWorkerId())
                                && message.getHeartbeat().getGeneration() == generation
                                && fencingToken.equals(message.getHeartbeat().getFencingToken()));
            }
        }

        private boolean hasTaskLogFrom(String workerId) {
            synchronized (messages) {
                return messages.stream()
                        .filter(Worker.WorkerMessage::hasTaskLog)
                        .anyMatch(message -> workerId.equals(message.getTaskLog().getWorkerId()));
            }
        }

        private boolean hasUnregister(String workerId, long generation, String fencingToken) {
            synchronized (messages) {
                return messages.stream()
                        .filter(Worker.WorkerMessage::hasUnregister)
                        .anyMatch(message -> workerId.equals(message.getUnregister().getWorkerId())
                                && message.getUnregister().getGeneration() == generation
                                && fencingToken.equals(message.getUnregister().getFencingToken()));
            }
        }
    }
}
