package net.tikeo.boot.autoconfigure;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import lombok.Getter;
import lombok.Setter;
import org.springframework.boot.context.properties.ConfigurationProperties;

/**
 * Spring Boot properties for tikeo workers.
 */
@Getter
@Setter
@ConfigurationProperties(prefix = "tikeo.worker")
public class TikeoWorkerProperties {
    /** Enable tikeo worker auto-configuration. */
    private boolean enabled = true;
    /** Auto-start the worker client with the Spring application lifecycle. */
    private boolean autoStartup = true;
    /** Tikeo Worker Tunnel endpoint. */
    private String endpoint = "http://0.0.0.0:9998";
    /** Dry-run mode avoids opening a live Worker Tunnel. */
    private boolean dryRun = false;
    /** Heartbeat interval in milliseconds. */
    private long heartbeatIntervalMillis = 10_000;
    /** Optional stable client-side instance hint; when blank, the SDK generates and persists one per scope and runtime identity. */
    private String clientInstanceId;
    /** Directory used to persist generated client instance ids. Blank uses ~/.tikeo/workers. */
    private String stateDir;
    /** Namespace reported during registration. */
    private String namespace = "default";
    /** App reported during registration. */
    private String app = "default";
    /** Cluster reported during registration. */
    private String cluster = "default";
    /** Region reported during registration. */
    private String region = "default";
    /** Capabilities reported during registration. */
    private List<String> capabilities = new ArrayList<>();
    /** Labels reported during registration. */
    private Map<String, String> labels = new LinkedHashMap<>();
    /** Worker-cluster master election settings. */
    private ElectionProperties election = new ElectionProperties();
    /** WASM sandbox runtime installation configuration. */
    private WasmProperties wasm = new WasmProperties();
    /** Dynamic script execution configuration. */
    private ScriptRunnerProperties scripts = new ScriptRunnerProperties();

    /** Worker cluster master election settings. */
    @Getter
    @Setter
    public static class ElectionProperties {
        /** Enable autonomous worker-cluster master election. */
        private boolean enabled = true;
        /** Optional explicit domain. Blank uses namespace/app/cluster/region. */
        private String domain = "";
        /** Deterministic priority; lower values win. */
        private int priority = 100;
    }

    /** Wasmtime installation settings for the default WASM sandbox. */
    @Getter
    @Setter
    public static class WasmProperties {
        /** Automatically install Wasmtime when it is unavailable. */
        private boolean autoInstall = true;
        /** Version passed to the official installer, for example latest or v45.0.0. */
        private String installVersion = "latest";
        /** Optional local install directory. Blank uses state-dir/wasmtime or ~/.tikeo/wasmtime. */
        private String installDir;
        /** Official installer URL. */
        private String installerUrl = "https://wasmtime.dev/install.sh";
        /** Installer timeout in milliseconds. */
        private long installTimeoutMillis = 120_000;
    }

    /** Dynamic script and optional container-backed non-WASM runner settings. */
    @Getter
    @Setter
    public static class ScriptRunnerProperties {
        /** Enable dynamic script execution through the default WASM sandbox. */
        private boolean enabled = true;
        /** Enable optional container-backed shell/python/node/powershell runners. */
        private boolean containerEnabled = false;
        /** Probe the container runtime before advertising non-WASM script capabilities. */
        private boolean availabilityCheck = true;
        /** Explicit Docker-compatible container runtime command for non-WASM scripts. */
        private String runtimeCommand = "";
        /** Extra runtime arguments appended before image. */
        private List<String> runtimeArgs = new ArrayList<>();
        /** Automatically install local development script runtime tools when absent. */
        private boolean autoInstallTools = true;
        /** Anthropic Sandbox Runtime npm package version. Blank/latest follows npm latest. */
        private String srtInstallVersion = "latest";
        /** Optional Anthropic Sandbox Runtime install directory. Blank uses state-dir/sandbox-tools/srt. */
        private String srtInstallDir = "";
        /** ripgrep crate version required by Anthropic Sandbox Runtime. Blank/latest follows cargo latest. */
        private String ripgrepInstallVersion = "latest";
        /** Optional ripgrep install directory. Blank uses state-dir/sandbox-tools/ripgrep. */
        private String ripgrepInstallDir = "";
        /** Deno version passed to the official installer; use latest by default. */
        private String denoInstallVersion = "latest";
        /** Optional Deno install directory. Blank uses state-dir/script-tools/deno or ~/.tikeo/script-tools/deno. */
        private String denoInstallDir = "";
        /** Official Deno installer URL. */
        private String denoInstallerUrl = "https://deno.land/install.sh";
        /** Rhai crate version for cargo install. Blank uses latest. */
        private String rhaiInstallVersion = "";
        /** Optional Rhai install directory. Blank uses state-dir/script-tools/rhai or ~/.tikeo/script-tools/rhai. */
        private String rhaiInstallDir = "";
        /** PowerShell Core version used when auto-installing pwsh for SRT-backed PowerShell. */
        private String powerShellInstallVersion = "7.5.4";
        /** Optional PowerShell Core install directory. Blank uses state-dir/sandbox-tools/pwsh. */
        private String powerShellInstallDir = "";
        /** Automatically install WasmEdge when absent. Disabled by default until explicitly selected. */
        private boolean wasmedgeAutoInstall = false;
        /** WasmEdge version passed to the installer; latest by default. */
        private String wasmedgeInstallVersion = "latest";
        /** Optional WasmEdge install directory. Blank uses state-dir/sandbox-tools/wasmedge. */
        private String wasmedgeInstallDir = "";
        /** WasmEdge installer URL. */
        private String wasmedgeInstallerUrl = "https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh";
        /** V8 runtime version. Blank/latest follows the installer default. */
        private String v8InstallVersion = "latest";
        /** Optional V8 runtime install directory. Blank uses state-dir/sandbox-tools/v8. */
        private String v8InstallDir = "";
        /** Script tool installer timeout in milliseconds. */
        private long toolInstallTimeoutMillis = 120_000;
        /** Per-language runtime images used inside the sandbox. */
        private ScriptRunnerImages images = new ScriptRunnerImages();
    }

    /** Per-language images for the container sandbox. */
    @Getter
    @Setter
    public static class ScriptRunnerImages {
        /** POSIX shell image. Blank disables shell scripts. */
        private String shell = "";
        /** Python image. Blank disables Python scripts. */
        private String python = "";
        /** JavaScript image. Blank disables JS scripts. */
        private String js = "";
        /** TypeScript image. Blank disables TS scripts. */
        private String ts = "";
        /** PowerShell image. Blank disables PowerShell scripts. */
        private String powershell = "";
        /** PHP image. Blank disables PHP scripts. */
        private String php = "";
        /** Groovy image. Blank disables Groovy scripts. */
        private String groovy = "";
        /** Rhai image. Blank disables Rhai scripts. */
        private String rhai = "";
    }
}
