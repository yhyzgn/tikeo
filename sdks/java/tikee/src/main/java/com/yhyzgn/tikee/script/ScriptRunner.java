package com.yhyzgn.tikee.script;

import com.yhyzgn.tikee.processor.TaskOutcome;

/** Worker-side sandbox runner for dynamic scripts. */
public interface ScriptRunner {
    ScriptRunnerKind kind();

    TaskOutcome run(ScriptRunnerTask task) throws Exception;
}
