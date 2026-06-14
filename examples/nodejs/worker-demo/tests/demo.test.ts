import { expect, test } from "bun:test";
import { installConsoleTaskLogBridge, processDispatchTask } from "@yhyzgn/tikeo";
import { processTask, scriptSandboxBackend } from "../src/main";

test("demo does not advertise local scripts by default", () => {
  expect(["1", "true", "yes", "on"].includes((process.env.TIKEO_ENABLE_LOCAL_SCRIPT_SHELL ?? "").toLowerCase())).toBe(false);
});

test("auto sandbox backend matches Java lightweight defaults", () => {
  delete process.env.TIKEO_WORKER_SCRIPT_SANDBOX;
  expect(scriptSandboxBackend("python")).toBe("srt");
  expect(scriptSandboxBackend("javascript")).toBe("deno");
  expect(scriptSandboxBackend("typescript")).toBe("deno");
});

test("demo processors emit native console logs through task bridge", async () => {
  const logs: [string, string][] = [];
  const bridge = installConsoleTaskLogBridge();
  try {
    const outcome = await processDispatchTask(processTask, undefined, {
      instanceId: "inst-1",
      jobId: "job-1",
      processorName: "demo.echo",
      payload: new TextEncoder().encode("hello"),
      assignmentToken: "assign-1",
    }, (level, message) => logs.push([level, message]));

    expect(outcome.success).toBe(true);
    expect(outcome.message).toBe("nodejs demo echo processed");
    expect(logs.some(([level, message]) => level === "info" && message.includes("[demo.echo]") && message.includes("hello"))).toBe(true);
  } finally {
    bridge.restore();
  }
});

test("demo fail and exception logs are bridged from console.error", async () => {
  const bridge = installConsoleTaskLogBridge();
  try {
    const failLogs: [string, string][] = [];
    const failure = await processDispatchTask(processTask, undefined, {
      instanceId: "inst-fail",
      jobId: "job-1",
      processorName: "demo.fail",
      payload: new TextEncoder().encode("bad-input"),
      assignmentToken: "assign-fail",
    }, (level, message) => failLogs.push([level, message]));
    expect(failure.success).toBe(false);
    expect(failure.message).toBe("nodejs demo intentional failure");
    expect(failLogs.some(([level, message]) => level === "error" && message.includes("[demo.fail]") && message.includes("bad-input"))).toBe(true);

    const exceptionLogs: [string, string][] = [];
    const exception = await processDispatchTask(processTask, undefined, {
      instanceId: "inst-exception",
      jobId: "job-1",
      processorName: "demo.exception",
      payload: new TextEncoder().encode("bad-input"),
      assignmentToken: "assign-exception",
    }, (level, message) => exceptionLogs.push([level, message]));
    expect(exception.success).toBe(false);
    expect(exception.message).toContain("nodejs demo runtime exception");
    expect(exceptionLogs.some(([level, message]) => level === "error" && message.includes("[demo.exception]") && message.includes("bad-input"))).toBe(true);
    expect(exceptionLogs.some(([level, message]) => level === "error" && message.includes("nodejs demo runtime exception") && message.includes("at"))).toBe(true);
  } finally {
    bridge.restore();
  }
});
