package com.yhyzgn.tikee.script;

/** Raised when a script runner cannot safely execute a dispatch. */
public class ScriptRunnerException extends RuntimeException {
    public ScriptRunnerException(String message) {
        super(message);
    }

    public ScriptRunnerException(String message, Throwable cause) {
        super(message, cause);
    }
}
