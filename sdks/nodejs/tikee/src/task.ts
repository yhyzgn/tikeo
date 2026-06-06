export type TaskLogSink = (level: string, message: string) => void;

export class TaskContext {
  constructor(
    public instanceId: string,
    public jobId: string,
    public processorName: string,
    public payload: Uint8Array = new Uint8Array(),
    public log?: TaskLogSink,
  ) {}

  logInfo(message: string): void { this.log?.("info", message); }
  logError(message: string): void { this.log?.("error", message); }
}

export interface TaskOutcome {
  success: boolean;
  message: string;
}

export type TaskProcessor = (task: TaskContext) => Promise<TaskOutcome> | TaskOutcome;

export function succeeded(message = ""): TaskOutcome { return { success: true, message }; }
export function failed(message: string): TaskOutcome { return { success: false, message }; }
