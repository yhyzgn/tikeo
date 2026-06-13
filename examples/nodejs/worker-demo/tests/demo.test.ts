import { expect, test } from "bun:test";
import { TaskContext } from "@yhyzgn/tikeo";
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

test("demo processors emit task logs", () => {
  const logs: [string, string][] = [];
  const outcome = processTask(new TaskContext("inst-1", "job-1", "demo.echo", new TextEncoder().encode("hello"), (level, message) => logs.push([level, message])));
  expect(outcome.success).toBe(true);
  expect(outcome.message).toBe("nodejs demo echo processed");
  expect(logs.some(([, message]) => message.includes("[demo.echo]"))).toBe(true);
});

test("demo fail returns business failure and demo exception throws runtime error", () => {
  const failLogs: [string, string][] = [];
  const failure = processTask(new TaskContext("inst-fail", "job-1", "demo.fail", new TextEncoder().encode("bad-input"), (level, message) => failLogs.push([level, message])));
  expect(failure.success).toBe(false);
  expect(failure.message).toBe("nodejs demo intentional failure");
  expect(failLogs.some(([level, message]) => level === "error" && message.includes("[demo.fail]") && message.includes("bad-input"))).toBe(true);

  const exceptionLogs: [string, string][] = [];
  expect(() => processTask(new TaskContext("inst-exception", "job-1", "demo.exception", new TextEncoder().encode("bad-input"), (level, message) => exceptionLogs.push([level, message])))).toThrow("nodejs demo runtime exception");
  expect(exceptionLogs.some(([level, message]) => level === "error" && message.includes("[demo.exception]") && message.includes("bad-input"))).toBe(true);
});
