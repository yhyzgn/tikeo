package tikeo

import (
	"context"
	"fmt"
	"io"
	"log"
	"log/slog"
	"strings"
)

type taskLogScopeKey struct{}

// TaskLogScope is the current job-instance logging sink carried in context.Context.
type TaskLogScope struct {
	InstanceID    string
	JobID         string
	ProcessorName string
	Log           func(level, message string)
}

// ContextWithTaskLogScope returns a child context carrying the current task log sink.
func ContextWithTaskLogScope(ctx context.Context, scope TaskLogScope) context.Context {
	if ctx == nil {
		ctx = context.Background()
	}
	return context.WithValue(ctx, taskLogScopeKey{}, scope)
}

// TaskLogScopeFromContext returns the active Tikeo task logging scope, if any.
func TaskLogScopeFromContext(ctx context.Context) (TaskLogScope, bool) {
	if ctx == nil {
		return TaskLogScope{}, false
	}
	scope, ok := ctx.Value(taskLogScopeKey{}).(TaskLogScope)
	return scope, ok && scope.Log != nil
}

// EmitTaskLogFromContext mirrors one log line into the current task scope.
func EmitTaskLogFromContext(ctx context.Context, level, message string) bool {
	scope, ok := TaskLogScopeFromContext(ctx)
	if !ok {
		return false
	}
	scope.Log(normalizeTaskLogLevel(level), message)
	return true
}

// TaskSlogHandler bridges log/slog records into the active Tikeo task scope.
type TaskSlogHandler struct {
	Next   slog.Handler
	attrs  []slog.Attr
	groups []string
}

func (h TaskSlogHandler) Enabled(ctx context.Context, level slog.Level) bool {
	if _, ok := TaskLogScopeFromContext(ctx); ok {
		return true
	}
	return h.Next != nil && h.Next.Enabled(ctx, level)
}

func (h TaskSlogHandler) Handle(ctx context.Context, record slog.Record) error {
	if scope, ok := TaskLogScopeFromContext(ctx); ok {
		scope.Log(slogLevel(record.Level), h.formatRecord(record))
	}
	if h.Next != nil && h.Next.Enabled(ctx, record.Level) {
		return h.Next.Handle(ctx, record)
	}
	return nil
}

func (h TaskSlogHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	next := h
	if h.Next != nil {
		next.Next = h.Next.WithAttrs(attrs)
	}
	next.attrs = append(append([]slog.Attr(nil), h.attrs...), attrs...)
	return next
}

func (h TaskSlogHandler) WithGroup(name string) slog.Handler {
	next := h
	if h.Next != nil {
		next.Next = h.Next.WithGroup(name)
	}
	if strings.TrimSpace(name) != "" {
		next.groups = append(append([]string(nil), h.groups...), name)
	}
	return next
}

func (h TaskSlogHandler) formatRecord(record slog.Record) string {
	parts := make([]string, 0, len(h.attrs)+record.NumAttrs()+1)
	if record.Message != "" {
		parts = append(parts, record.Message)
	}
	for _, attr := range h.attrs {
		parts = append(parts, formatSlogAttr(h.groups, attr))
	}
	record.Attrs(func(attr slog.Attr) bool {
		parts = append(parts, formatSlogAttr(h.groups, attr))
		return true
	})
	return strings.Join(parts, " ")
}

func formatSlogAttr(groups []string, attr slog.Attr) string {
	attr.Value = attr.Value.Resolve()
	key := attr.Key
	if len(groups) > 0 {
		key = strings.Join(append(append([]string(nil), groups...), key), ".")
	}
	return fmt.Sprintf("%s=%v", key, attr.Value.Any())
}

// NewTaskLogger returns a standard-library log.Logger that writes into the current task scope.
func NewTaskLogger(ctx context.Context, prefix string, flag int) *log.Logger {
	return log.New(taskLogWriter{ctx: ctx}, prefix, flag)
}

type taskLogWriter struct{ ctx context.Context }

func (w taskLogWriter) Write(p []byte) (int, error) {
	message := strings.TrimRight(string(p), "\r\n")
	EmitTaskLogFromContext(w.ctx, "info", message)
	return len(p), nil
}

var _ io.Writer = taskLogWriter{}

func slogLevel(level slog.Level) string {
	if level >= slog.LevelError {
		return "error"
	}
	if level >= slog.LevelWarn {
		return "warning"
	}
	if level <= slog.LevelDebug {
		return "debug"
	}
	return "info"
}

func normalizeTaskLogLevel(level string) string {
	switch strings.ToLower(strings.TrimSpace(level)) {
	case "debug":
		return "debug"
	case "warn", "warning":
		return "warning"
	case "error":
		return "error"
	default:
		return "info"
	}
}
