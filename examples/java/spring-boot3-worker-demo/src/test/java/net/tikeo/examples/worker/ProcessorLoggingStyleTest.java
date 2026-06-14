package net.tikeo.examples.worker;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class ProcessorLoggingStyleTest {
    @Test
    void processorsUseFrameworkLoggerInsteadOfManualContextTaskLogs() throws Exception {
        Path processorDir = Path.of("src/main/java/net/tikeo/examples/worker/processor");
        List<Path> processors;
        try (var stream = Files.list(processorDir)) {
            processors = stream
                    .filter(path -> path.getFileName().toString().endsWith("TaskProcessor.java"))
                    .toList();
        }

        for (Path processor : processors) {
            String source = Files.readString(processor);
            assertEquals(false, source.contains("context.logInfo("), processor + " should use log.info for bridge capture");
            assertEquals(false, source.contains("context.logError("), processor + " should use log.error for bridge capture");
        }
    }
}
