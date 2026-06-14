export type TaskLogBridgeSink = (level: string, message: string) => void;
export interface TaskLogScope {
    instanceId: string;
    jobId: string;
    processorName: string;
    log: TaskLogBridgeSink;
}
export declare function currentTaskLogScope(): TaskLogScope | undefined;
export declare function runWithTaskLogScope<T>(scope: TaskLogScope, fn: () => T): T;
export declare function emitCurrentTaskLog(level: string, message: string): boolean;
export interface ConsoleTaskLogBridge {
    restore(): void;
}
export declare function runWithoutTaskLogBridgeCapture<T>(fn: () => T): T;
/**
 * Bridge Node's built-in console methods into the active Tikeo task scope.
 *
 * The bridge is task-scoped through AsyncLocalStorage: console output emitted outside a running
 * task is left untouched, while console.log/info/warn/error inside a processor is also sent as
 * TaskLog. This is a bridge, not stdout/stderr scraping, so unrelated process output is not attached
 * to job instances.
 */
export declare function installConsoleTaskLogBridge(): ConsoleTaskLogBridge;
/** Minimal logger object for demos or small applications that do not use pino/winston yet. */
export declare const taskLogger: {
    debug: (...args: unknown[]) => boolean;
    info: (...args: unknown[]) => boolean;
    warn: (...args: unknown[]) => boolean;
    error: (...args: unknown[]) => boolean;
};
