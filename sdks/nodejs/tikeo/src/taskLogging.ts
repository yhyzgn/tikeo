import { AsyncLocalStorage } from "node:async_hooks";
import { formatWithOptions } from "node:util";

export type TaskLogBridgeSink = (level: string, message: string) => void;

export interface TaskLogScope {
  instanceId: string;
  jobId: string;
  processorName: string;
  log: TaskLogBridgeSink;
}

const taskLogStorage = new AsyncLocalStorage<TaskLogScope>();
let taskLogBridgeMuted = false;

export function currentTaskLogScope(): TaskLogScope | undefined {
  return taskLogStorage.getStore();
}

export function runWithTaskLogScope<T>(scope: TaskLogScope, fn: () => T): T {
  return taskLogStorage.run(scope, fn);
}

export function emitCurrentTaskLog(level: string, message: string): boolean {
  if (taskLogBridgeMuted) return false;
  const scope = currentTaskLogScope();
  if (!scope) return false;
  scope.log(level || "info", message);
  return true;
}

export interface ConsoleTaskLogBridge {
  restore(): void;
}

export function runWithoutTaskLogBridgeCapture<T>(fn: () => T): T {
  const wasMuted = taskLogBridgeMuted;
  taskLogBridgeMuted = true;
  try {
    return fn();
  } finally {
    taskLogBridgeMuted = wasMuted;
  }
}

let consoleBridge: ConsoleTaskLogBridge | undefined;

/**
 * Bridge Node's built-in console methods into the active Tikeo task scope.
 *
 * The bridge is task-scoped through AsyncLocalStorage: console output emitted outside a running
 * task is left untouched, while console.log/info/warn/error inside a processor is also sent as
 * TaskLog. This is a bridge, not stdout/stderr scraping, so unrelated process output is not attached
 * to job instances.
 */
export function installConsoleTaskLogBridge(): ConsoleTaskLogBridge {
  if (consoleBridge) return consoleBridge;
  const original = {
    log: console.log,
    info: console.info,
    warn: console.warn,
    error: console.error,
  };
  const wrap = (level: string, method: (...args: unknown[]) => void) => (...args: unknown[]) => {
    const message = formatWithOptions({ colors: false }, ...args);
    emitCurrentTaskLog(level, message);
    method(...args);
  };
  console.log = wrap("info", original.log);
  console.info = wrap("info", original.info);
  console.warn = wrap("warning", original.warn);
  console.error = wrap("error", original.error);
  consoleBridge = {
    restore() {
      console.log = original.log;
      console.info = original.info;
      console.warn = original.warn;
      console.error = original.error;
      consoleBridge = undefined;
    },
  };
  return consoleBridge;
}

/** Minimal logger object for demos or small applications that do not use pino/winston yet. */
export const taskLogger = {
  debug: (...args: unknown[]) => emitCurrentTaskLog("debug", formatWithOptions({ colors: false }, ...args)),
  info: (...args: unknown[]) => emitCurrentTaskLog("info", formatWithOptions({ colors: false }, ...args)),
  warn: (...args: unknown[]) => emitCurrentTaskLog("warning", formatWithOptions({ colors: false }, ...args)),
  error: (...args: unknown[]) => emitCurrentTaskLog("error", formatWithOptions({ colors: false }, ...args)),
};
