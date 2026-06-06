package tikee

import "context"

// TaskContext is the ergonomic task shape passed to Go processors.
type TaskContext struct {
	InstanceID    string
	JobID         string
	ProcessorName string
	Payload       []byte
	Log           func(level, message string)
}

// LogInfo emits one task-scoped info log line. It is precise to the current task instance.
func (t TaskContext) LogInfo(message string) {
	t.log("info", message)
}

// LogError emits one task-scoped error log line. It is precise to the current task instance.
func (t TaskContext) LogError(message string) {
	t.log("error", message)
}

func (t TaskContext) log(level string, message string) {
	if t.Log != nil {
		t.Log(level, message)
	}
}

// TaskOutcome is the worker result reported to tikee.
type TaskOutcome struct {
	Success bool
	Message string
}

// Succeeded returns a successful task outcome.
func Succeeded() TaskOutcome {
	return TaskOutcome{Success: true}
}

// Failed returns a failed task outcome with an operator-facing message.
func Failed(message string) TaskOutcome {
	return TaskOutcome{Success: false, Message: message}
}

// TaskProcessor processes one assigned task.
type TaskProcessor interface {
	Process(context.Context, TaskContext) (TaskOutcome, error)
}

// TaskProcessorFunc adapts a function into a TaskProcessor.
type TaskProcessorFunc func(context.Context, TaskContext) (TaskOutcome, error)

// Process executes f.
func (f TaskProcessorFunc) Process(ctx context.Context, task TaskContext) (TaskOutcome, error) {
	return f(ctx, task)
}
