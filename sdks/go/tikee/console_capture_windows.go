//go:build windows

package tikee

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"strings"
	"sync"
)

var taskConsoleCaptureMu sync.Mutex

func captureTaskConsoleLogs(emitLog func(level, message string), run func() (TaskOutcome, error)) (TaskOutcome, error) {
	if emitLog == nil {
		return run()
	}
	taskConsoleCaptureMu.Lock()
	defer taskConsoleCaptureMu.Unlock()

	originalOut := os.Stdout
	originalErr := os.Stderr
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
	done := make(chan struct{}, 2)
	go mirrorTaskConsoleStream(stdoutReader, originalOut, "info", emitLog, done)
	go mirrorTaskConsoleStream(stderrReader, originalErr, "error", emitLog, done)
	os.Stdout = stdoutWriter
	os.Stderr = stderrWriter
	outcome, runErr := run()
	os.Stdout = originalOut
	os.Stderr = originalErr
	_ = stdoutWriter.Close()
	_ = stderrWriter.Close()
	<-done
	<-done
	return outcome, runErr
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
