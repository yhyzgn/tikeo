package net.tikeo.script;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Single source of truth for task-scoped script sandbox HOME/TMPDIR/XDG/runtime paths.
 */
final class TaskRuntimeDirs implements AutoCloseable {
    private static final AtomicLong SCRIPT_FILE_SEQUENCE = new AtomicLong();
    private static final Set<String> MANAGED_ENV_NAMES = Set.of(
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "TMPDIR",
        "TERM",
        "NO_COLOR",
        "CLAUDE_CODE_TMPDIR",
        "CLAUDE_TMPDIR",
        "PSModulePath",
        "DOTNET_CLI_HOME",
        "POWERSHELL_TELEMETRY_OPTOUT",
        "POWERSHELL_UPDATECHECK",
        "DENO_DIR"
    );

    private final Path root;
    private final Path home;
    private final Path config;
    private final Path cache;
    private final Path data;
    private final Path modules;
    private final Path dotnetHome;
    private final Path tmp;
    private final Path denoDir;

    private TaskRuntimeDirs(Path root) {
        this.root = root;
        this.home = root.resolve("home");
        this.config = root.resolve("config");
        this.cache = root.resolve("cache");
        this.data = root.resolve("data");
        this.modules = data.resolve("powershell").resolve("Modules");
        this.dotnetHome = root.resolve("dotnet");
        this.tmp = root.resolve("tmp");
        this.denoDir = cache.resolve("deno");
    }

    static TaskRuntimeDirs create(String prefix) throws IOException {
        TaskRuntimeDirs dirs = new TaskRuntimeDirs(Files.createTempDirectory(prefix + "-"));
        for (Path directory : dirs.requiredDirectories()) {
            Files.createDirectories(directory);
        }
        return dirs;
    }

    private List<Path> requiredDirectories() {
        return List.of(root, home, config, cache, data, modules, dotnetHome, tmp, denoDir);
    }

    List<String> writablePaths() {
        return List.of(root, home, config, cache, data, dotnetHome, tmp, denoDir)
            .stream()
            .map(Path::toString)
            .toList();
    }

    Path workingDir() {
        return home;
    }

    Path scriptFile(String extension) {
        return home.resolve(
            "script-" + System.currentTimeMillis() + "-" + SCRIPT_FILE_SEQUENCE.getAndIncrement() + "." + extension
        );
    }

    void applySrtEnvironment(ProcessBuilder builder, List<String> extraPathEntries) {
        Map<String, String> env = builder.environment();
        env.putAll(baseEnvironment(extraPathEntries));
        env.put("CLAUDE_CODE_TMPDIR", tmp.toString());
        env.put("CLAUDE_TMPDIR", tmp.toString());
    }

    void applyPowerShellEnvironment(ProcessBuilder builder) {
        Map<String, String> env = builder.environment();
        env.put("PSModulePath", modules.toString());
        env.put("DOTNET_CLI_HOME", dotnetHome.toString());
        env.put("POWERSHELL_TELEMETRY_OPTOUT", "1");
        env.put("POWERSHELL_UPDATECHECK", "Off");
    }

    void applyDenoEnvironment(ProcessBuilder builder) {
        Map<String, String> env = builder.environment();
        env.putAll(baseEnvironment(List.of()));
        env.put("DENO_DIR", denoDir.toString());
    }

    private Map<String, String> baseEnvironment(List<String> extraPathEntries) {
        java.util.LinkedHashMap<String, String> env = new java.util.LinkedHashMap<>();
        env.put("HOME", home.toString());
        env.put("XDG_CONFIG_HOME", config.toString());
        env.put("XDG_CACHE_HOME", cache.toString());
        env.put("XDG_DATA_HOME", data.toString());
        env.put("TMPDIR", tmp.toString());
        env.put("TERM", "dumb");
        env.put("NO_COLOR", "1");
        String path = mergedPath(extraPathEntries);
        if (!path.isBlank()) {
            env.put("PATH", path);
        }
        return env;
    }

    void appendAllowedUnmanagedEnv(ProcessBuilder builder, List<String> allowedEnvVars) {
        Map<String, String> env = builder.environment();
        for (String name : allowedEnvVars) {
            if (name == null || name.isBlank() || isManagedEnvironmentName(name)) {
                continue;
            }
            String value = System.getenv(name);
            if (value != null) {
                env.put(name, value);
            }
        }
    }

    static boolean isManagedEnvironmentName(String name) {
        return MANAGED_ENV_NAMES.contains(name);
    }

    Path root() {
        return root;
    }

    Path home() {
        return home;
    }

    Path tmp() {
        return tmp;
    }

    Path denoDir() {
        return denoDir;
    }

    @Override
    public void close() {
        deleteRecursively(root);
    }

    private static String mergedPath(List<String> extraPathEntries) {
        LinkedHashSet<String> entries = new LinkedHashSet<>();
        for (String entry : extraPathEntries) {
            if (entry != null && !entry.isBlank()) {
                entries.add(entry);
            }
        }
        String existing = System.getenv("PATH");
        if (existing != null && !existing.isBlank()) {
            entries.addAll(List.of(existing.split(java.util.regex.Pattern.quote(java.io.File.pathSeparator))));
        }
        return String.join(java.io.File.pathSeparator, entries);
    }

    private static void deleteRecursively(Path path) {
        if (path == null || !Files.exists(path)) {
            return;
        }
        try (java.util.stream.Stream<Path> stream = Files.walk(path)) {
            stream.sorted(java.util.Comparator.reverseOrder()).forEach(TaskRuntimeDirs::deleteQuietly);
        } catch (IOException ignored) {
            // Best-effort cleanup only.
        }
    }

    private static void deleteQuietly(Path path) {
        try {
            Files.deleteIfExists(path);
        } catch (IOException ignored) {
            // Best-effort cleanup only.
        }
    }
}
