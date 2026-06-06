import { expect, test } from "bun:test";
import { TaskContext } from "@yhyzgn/tikee";
import { processTask, scriptSandboxBackend } from "../src/main";

test("demo does not advertise local scripts by default", () => {
  expect(["1", "true", "yes", "on"].includes((process.env.TIKEE_ENABLE_LOCAL_SCRIPT_SHELL ?? "").toLowerCase())).toBe(false);
});

test("auto sandbox backend matches Java lightweight defaults", () => {
  delete process.env.TIKEE_WORKER_SCRIPT_SANDBOX;
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
