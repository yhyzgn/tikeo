package net.tikeo.script;

import java.util.List;

/**
 * Default-deny policy snapshot attached to a released script dispatch.
 */
public record ScriptRunnerPolicy(
        long timeoutMillis,
        long maxMemoryBytes,
        long maxOutputBytes,
        boolean allowNetwork,
        List<String> allowedNetworkHosts,
        List<String> allowedEnvVars,
        List<String> readOnlyPaths,
        List<String> writablePaths,
        List<String> secretRefs) {
    public ScriptRunnerPolicy {
        allowedNetworkHosts = List.copyOf(allowedNetworkHosts == null ? List.of() : allowedNetworkHosts);
        allowedEnvVars = List.copyOf(allowedEnvVars == null ? List.of() : allowedEnvVars);
        readOnlyPaths = List.copyOf(readOnlyPaths == null ? List.of() : readOnlyPaths);
        writablePaths = List.copyOf(writablePaths == null ? List.of() : writablePaths);
        secretRefs = List.copyOf(secretRefs == null ? List.of() : secretRefs);
    }

    public void validateResourceLimits() {
        if (timeoutMillis <= 0) {
            throw new ScriptRunnerException("script timeout must be greater than zero");
        }
        if (maxMemoryBytes <= 0) {
            throw new ScriptRunnerException("script memory limit must be greater than zero");
        }
        if (maxOutputBytes <= 0) {
            throw new ScriptRunnerException("script output limit must be greater than zero");
        }
    }
}
