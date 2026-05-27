package com.yhyzgn.tikee.script;

import java.util.Locale;
import java.util.Optional;

/** Supported dynamic script runner kinds. */
public enum ScriptRunnerKind {
    /** POSIX shell scripts. */
    SHELL("shell"),
    /** Python scripts. */
    PYTHON("python"),
    /** JavaScript scripts. */
    JS("js"),
    /** TypeScript scripts. */
    TS("ts"),
    /** PowerShell scripts. */
    POWERSHELL("powershell");

    private final String value;

    ScriptRunnerKind(String value) {
        this.value = value;
    }

    public String value() {
        return value;
    }

    public String capability() {
        return "script:" + value;
    }

    public static Optional<ScriptRunnerKind> fromLanguage(String language) {
        String normalized = language == null ? "" : language.trim().toLowerCase(Locale.ROOT);
        return switch (normalized) {
            case "shell", "sh", "bash" -> Optional.of(SHELL);
            case "python", "py" -> Optional.of(PYTHON);
            case "node", "nodejs", "javascript", "js" -> Optional.of(JS);
            case "typescript", "ts" -> Optional.of(TS);
            case "powershell", "pwsh" -> Optional.of(POWERSHELL);
            default -> Optional.empty();
        };
    }
}
