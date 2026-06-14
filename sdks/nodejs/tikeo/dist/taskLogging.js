import { AsyncLocalStorage } from "node:async_hooks";
import { formatWithOptions } from "node:util";
const taskLogStorage = new AsyncLocalStorage();
export function currentTaskLogScope() {
    return taskLogStorage.getStore();
}
export function runWithTaskLogScope(scope, fn) {
    return taskLogStorage.run(scope, fn);
}
export function emitCurrentTaskLog(level, message) {
    const scope = currentTaskLogScope();
    if (!scope)
        return false;
    scope.log(level || "info", message);
    return true;
}
let consoleBridge;
/**
 * Bridge Node's built-in console methods into the active Tikeo task scope.
 *
 * The bridge is task-scoped through AsyncLocalStorage: console output emitted outside a running
 * task is left untouched, while console.log/info/warn/error inside a processor is also sent as
 * TaskLog. This is a bridge, not stdout/stderr scraping, so unrelated process output is not attached
 * to job instances.
 */
export function installConsoleTaskLogBridge() {
    if (consoleBridge)
        return consoleBridge;
    const original = {
        log: console.log,
        info: console.info,
        warn: console.warn,
        error: console.error,
    };
    const wrap = (level, method) => (...args) => {
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
    debug: (...args) => emitCurrentTaskLog("debug", formatWithOptions({ colors: false }, ...args)),
    info: (...args) => emitCurrentTaskLog("info", formatWithOptions({ colors: false }, ...args)),
    warn: (...args) => emitCurrentTaskLog("warning", formatWithOptions({ colors: false }, ...args)),
    error: (...args) => emitCurrentTaskLog("error", formatWithOptions({ colors: false }, ...args)),
};
