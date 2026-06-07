import { appendFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

/** SDK diagnostic severity. */
export type SdkLogLevel = "debug" | "info" | "warning" | "error";

/** Application-owned bridge for SDK diagnostics. */
export interface SdkLogger {
  /**
   * Emit one SDK diagnostic message.
   *
   * This method is for worker lifecycle diagnostics only. Task output must continue to use the
   * task-scoped logger so unrelated process messages are never attached to a job instance.
   */
  log(level: SdkLogLevel, message: string): void;
}

/** SDK diagnostic logger configuration. */
export interface SdkLogConfig {
  /** Minimum emitted level. Defaults to `info`. */
  level?: SdkLogLevel | string;
  /** Optional directory that receives `tikeo-sdk.log` in addition to console output. */
  logDir?: string;
}

const weights: Record<SdkLogLevel, number> = { debug: 0, info: 1, warning: 2, error: 3 };

/** Parse a user-facing log-level name. Unknown names fall back to `info`. */
export function parseSdkLogLevel(level: string | undefined): SdkLogLevel {
  const normalized = (level ?? "info").trim().toLowerCase();
  if (normalized === "debug" || normalized === "info" || normalized === "error") return normalized;
  if (normalized === "warn" || normalized === "warning") return "warning";
  return "info";
}

class DefaultSdkLogger implements SdkLogger {
  private readonly level: SdkLogLevel;
  private readonly logFile?: string;

  constructor(config: SdkLogConfig = {}) {
    this.level = parseSdkLogLevel(config.level);
    const logDir = config.logDir?.trim();
    if (logDir) {
      mkdirSync(logDir, { recursive: true });
      this.logFile = join(logDir, "tikeo-sdk.log");
    }
  }

  log(level: SdkLogLevel, message: string): void {
    if (weights[level] < weights[this.level]) return;
    const line = `[tikeo-sdk] ${level} ${message}`;
    if (level === "error") console.error(line);
    else console.log(line);
    if (this.logFile) appendFileSync(this.logFile, `${line}\n`, "utf8");
  }
}

let logger: SdkLogger = new DefaultSdkLogger({
  level: process.env.TIKEO_SDK_LOG_LEVEL,
  logDir: process.env.TIKEO_SDK_LOG_DIR,
});

/**
 * Configure SDK diagnostics for console and optional file output.
 *
 * Call this during worker startup. The logger does not capture stdout/stderr and therefore cannot
 * pollute task instance logs with unrelated application messages.
 */
export function configureSdkLogging(config: SdkLogConfig = {}): void {
  logger = new DefaultSdkLogger(config);
}

/** Bridge SDK diagnostics into an application-owned logger. */
export function setSdkLogger(next: SdkLogger): void {
  logger = next;
}

/** Emit one SDK diagnostic event. */
export function sdkLog(level: SdkLogLevel, message: string): void {
  logger.log(level, message);
}
