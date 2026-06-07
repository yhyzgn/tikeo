package tikeo

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

// LogLevel controls SDK diagnostic verbosity.
//
// SDK diagnostics are process/runtime logs for worker connectivity, registration, heartbeat,
// sandbox setup, and management API calls. They are intentionally separate from task-scoped
// instance logs, which must be emitted through TaskContext so unrelated process output is not
// attached to a job instance.
type LogLevel int

const (
	// LogLevelDebug emits verbose troubleshooting diagnostics.
	LogLevelDebug LogLevel = iota
	// LogLevelInfo emits normal lifecycle diagnostics and is the default.
	LogLevelInfo
	// LogLevelWarning emits recoverable operational problems.
	LogLevelWarning
	// LogLevelError emits failed runtime operations.
	LogLevelError
)

// ParseLogLevel parses a user-facing log-level name. Unknown names fall back to INFO.
func ParseLogLevel(value string) LogLevel {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "debug":
		return LogLevelDebug
	case "warn", "warning":
		return LogLevelWarning
	case "error":
		return LogLevelError
	default:
		return LogLevelInfo
	}
}

func (l LogLevel) String() string {
	switch l {
	case LogLevelDebug:
		return "debug"
	case LogLevelWarning:
		return "warning"
	case LogLevelError:
		return "error"
	default:
		return "info"
	}
}

// Logger receives SDK diagnostic events.
//
// Implement this interface to bridge tikeo diagnostics into zap, slog, logrus, or an existing
// platform logger without coupling the SDK to any logging framework. Implementations must not
// forward these diagnostics as task instance logs.
type Logger interface {
	Log(level LogLevel, message string)
}

// LogConfig configures the default SDK logger.
type LogConfig struct {
	// Level is the minimum diagnostic severity. The default is INFO.
	Level LogLevel
	// LogDir optionally receives tikeo-sdk.log in addition to console output.
	LogDir string
	// Writer overrides console output when tests or applications need full control.
	Writer io.Writer
}

// DefaultLogConfig returns INFO-level console logging with no file output.
func DefaultLogConfig() LogConfig {
	return LogConfig{Level: LogLevelInfo, Writer: os.Stdout}
}

// LogConfigFromEnv reads TIKEO_SDK_LOG_LEVEL and TIKEO_SDK_LOG_DIR.
func LogConfigFromEnv() LogConfig {
	config := DefaultLogConfig()
	if value := strings.TrimSpace(os.Getenv("TIKEO_SDK_LOG_LEVEL")); value != "" {
		config.Level = ParseLogLevel(value)
	}
	config.LogDir = strings.TrimSpace(os.Getenv("TIKEO_SDK_LOG_DIR"))
	return config
}

type defaultLogger struct {
	mu     sync.Mutex
	level  LogLevel
	writer io.Writer
	file   *os.File
}

func newDefaultLogger(config LogConfig) Logger {
	writer := config.Writer
	if writer == nil {
		writer = os.Stdout
	}
	logger := &defaultLogger{level: config.Level, writer: writer}
	if strings.TrimSpace(config.LogDir) != "" {
		if err := os.MkdirAll(config.LogDir, 0o755); err == nil {
			if file, err := os.OpenFile(filepath.Join(config.LogDir, "tikeo-sdk.log"), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644); err == nil {
				logger.file = file
			}
		}
	}
	return logger
}

func (l *defaultLogger) Log(level LogLevel, message string) {
	if level < l.level {
		return
	}
	line := fmt.Sprintf("[tikeo-sdk] %s %s\n", level.String(), message)
	l.mu.Lock()
	defer l.mu.Unlock()
	_, _ = io.WriteString(l.writer, line)
	if l.file != nil {
		_, _ = io.WriteString(l.file, line)
	}
}

var sdkLogger = newDefaultLogger(LogConfigFromEnv())

// ConfigureLogging installs the process-level SDK diagnostic logger.
//
// Call this once during worker startup. The logger writes SDK lifecycle diagnostics only; task
// output must still go through TaskContext.LogInfo or TaskContext.LogError.
func ConfigureLogging(config LogConfig) {
	sdkLogger = newDefaultLogger(config)
}

// SetLogger bridges SDK diagnostics into an application-owned logger.
func SetLogger(logger Logger) {
	if logger != nil {
		sdkLogger = logger
	}
}

func sdkLog(level LogLevel, format string, args ...any) {
	if sdkLogger != nil {
		sdkLogger.Log(level, fmt.Sprintf(format, args...))
	}
}
