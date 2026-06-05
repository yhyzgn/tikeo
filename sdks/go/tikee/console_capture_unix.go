//go:build unix

package tikee

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"strings"
	"sync"

	"golang.org/x/sys/unix"
)

var taskConsoleCaptureMu sync.Mutex

func captureTaskConsoleLogs(emitLog func(level, message string), run func() (TaskOutcome, error)) (TaskOutcome, error) {
	if emitLog == nil {
		return run()
	}
	taskConsoleCaptureMu.Lock()
	defer taskConsoleCaptureMu.Unlock()

	stdoutReader, stdoutWriter, err := os.Pipe()
	if err != nil {
		return TaskOutcome{}, fmt.Errorf("capture stdout pipe failed: %w", err)
	}
	stderrReader, stderrWriter, err := os.Pipe()
	if err != nil {
		_ = stdoutReader.Close()
		_ = stdoutWriter.Close()
		return TaskOutcome{}, fmt.Errorf("capture stderr pipe failed: %w", err)
	}
	originalStdout, err := unix.Dup(int(os.Stdout.Fd()))
	if err != nil {
		_ = stdoutReader.Close()
		_ = stdoutWriter.Close()
		_ = stderrReader.Close()
		_ = stderrWriter.Close()
		return TaskOutcome{}, fmt.Errorf("capture stdout dup failed: %w", err)
	}
	originalStderr, err := unix.Dup(int(os.Stderr.Fd()))
	if err != nil {
		_ = unix.Close(originalStdout)
		_ = stdoutReader.Close()
		_ = stdoutWriter.Close()
		_ = stderrReader.Close()
		_ = stderrWriter.Close()
		return TaskOutcome{}, fmt.Errorf("capture stderr dup failed: %w", err)
	}
	originalOutFile := os.NewFile(uintptr(originalStdout), "tikee-original-stdout")
	originalErrFile := os.NewFile(uintptr(originalStderr), "tikee-original-stderr")

	done := make(chan struct{}, 2)
	go mirrorTaskConsoleStream(stdoutReader, originalOutFile, "info", emitLog, done)
	go mirrorTaskConsoleStream(stderrReader, originalErrFile, "error", emitLog, done)

	if err := unix.Dup2(int(stdoutWriter.Fd()), int(os.Stdout.Fd())); err != nil {
		return restoreFailedConsole(originalOutFile, originalErrFile, stdoutReader, stdoutWriter, stderrReader, stderrWriter, err, "stdout")
	}
	if err := unix.Dup2(int(stderrWriter.Fd()), int(os.Stderr.Fd())); err != nil {
		return restoreFailedConsole(originalOutFile, originalErrFile, stdoutReader, stdoutWriter, stderrReader, stderrWriter, err, "stderr")
	}

	outcome, runErr := run()
	_ = unix.Dup2(originalStdout, int(os.Stdout.Fd()))
	_ = unix.Dup2(originalStderr, int(os.Stderr.Fd()))
	_ = stdoutWriter.Close()
	_ = stderrWriter.Close()
	<-done
	<-done
	_ = originalOutFile.Close()
	_ = originalErrFile.Close()
	return outcome, runErr
}

func restoreFailedConsole(
	originalOutFile *os.File,
	originalErrFile *os.File,
	stdoutReader *os.File,
	stdoutWriter *os.File,
	stderrReader *os.File,
	stderrWriter *os.File,
	error error,
	stream string,
) (TaskOutcome, error) {
	_ = unix.Dup2(int(originalOutFile.Fd()), int(os.Stdout.Fd()))
	_ = unix.Dup2(int(originalErrFile.Fd()), int(os.Stderr.Fd()))
	_ = originalOutFile.Close()
	_ = originalErrFile.Close()
	_ = stdoutReader.Close()
	_ = stdoutWriter.Close()
	_ = stderrReader.Close()
	_ = stderrWriter.Close()
	return TaskOutcome{}, fmt.Errorf("capture %s redirect failed: %w", stream, error)
}

func mirrorTaskConsoleStream(reader *os.File, original io.Writer, level string, emitLog func(level, message string), done chan<- struct{}) {
	defer func() {
		_ = reader.Close()
		done <- struct{}{}
	}()
	var line bytes.Buffer
	buffer := make([]byte, 4096)
	for {
		count, err := reader.Read(buffer)
		if count > 0 {
			chunk := buffer[:count]
			_, _ = original.Write(chunk)
			for _, value := range chunk {
				if value == '\n' {
					emitCapturedConsoleLine(level, line.String(), emitLog)
					line.Reset()
					continue
				}
				if value != '\r' {
					_ = line.WriteByte(value)
				}
			}
		}
		if err != nil {
			if line.Len() > 0 {
				emitCapturedConsoleLine(level, line.String(), emitLog)
			}
			return
		}
	}
}

func emitCapturedConsoleLine(level string, line string, emitLog func(level, message string)) {
	trimmed := strings.TrimSpace(line)
	if trimmed != "" {
		emitLog(level, trimmed)
	}
}
