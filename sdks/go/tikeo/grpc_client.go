package tikeo

import (
	"context"
	"errors"
	"fmt"
	"net/url"
	"os"
	"runtime/debug"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/yhyzgn/tikeo/sdks/go/tikeo/internal/workerpb"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// WorkerTunnelClient is the official generated gRPC Worker Tunnel client type.
type WorkerTunnelClient = workerpb.WorkerTunnelServiceClient

// DialOption customizes the real Worker Tunnel connection.
type DialOption func(*dialConfig)

type dialConfig struct {
	grpcOptions []grpc.DialOption
}

// WithGRPCDialOption appends one official google.golang.org/grpc dial option.
func WithGRPCDialOption(option grpc.DialOption) DialOption {
	return func(config *dialConfig) {
		config.grpcOptions = append(config.grpcOptions, option)
	}
}

// ConnectGRPC validates config and opens a grpc.ClientConn with the official Go gRPC library.
func (c *Client) ConnectGRPC(ctx context.Context, options ...DialOption) (*grpc.ClientConn, error) {
	sdkLog(LogLevelInfo, "connecting worker tunnel endpoint=%s client_instance_id=%s", c.config.Endpoint, c.config.ClientInstanceID)
	if err := c.config.Validate(); err != nil {
		return nil, err
	}
	if ctx == nil {
		return nil, errors.New("tikeo grpc dial context is required")
	}
	if err := ctx.Err(); err != nil {
		return nil, fmt.Errorf("tikeo grpc dial context is not usable: %w", err)
	}
	target, err := grpcTarget(c.config.Endpoint)
	if err != nil {
		return nil, err
	}
	config := dialConfig{grpcOptions: []grpc.DialOption{grpc.WithTransportCredentials(insecure.NewCredentials())}}
	for _, option := range options {
		if option != nil {
			option(&config)
		}
	}
	conn, err := grpc.NewClient(target, config.grpcOptions...)
	if err != nil {
		return nil, fmt.Errorf("tikeo grpc client create failed: %w", err)
	}
	conn.Connect()
	sdkLog(LogLevelDebug, "grpc connection created target=%s", target)
	return conn, nil
}

func grpcTarget(endpoint string) (string, error) {
	value := strings.TrimSpace(endpoint)
	if value == "" {
		return "", errors.New("tikeo worker endpoint is required")
	}

	parsed, err := url.Parse(value)
	if err != nil {
		if strings.Contains(value, "://") {
			return "", fmt.Errorf("tikeo worker endpoint is invalid: %w", err)
		}
		return value, nil
	}
	if parsed.Scheme == "http" || parsed.Scheme == "https" {
		if parsed.Host == "" {
			return "", errors.New("tikeo worker endpoint host is required")
		}
		return parsed.Host, nil
	}

	return value, nil
}

// NewWorkerTunnelClient returns the generated Worker Tunnel gRPC client for a connection.
func NewWorkerTunnelClient(conn grpc.ClientConnInterface) WorkerTunnelClient {
	return workerpb.NewWorkerTunnelServiceClient(conn)
}

// Session is an active Worker Tunnel registration.
type Session struct {
	conn           *grpc.ClientConn
	stream         workerpb.WorkerTunnelService_OpenTunnelClient
	sendMu         sync.Mutex
	workerID       string
	leaseSeconds   uint64
	generation     uint64
	fencingToken   string
	heartbeatEvery time.Duration
	sequence       uint64
	logSequence    int64
}

func (c *Client) Connect(ctx context.Context, options ...DialOption) (*Session, error) {
	conn, err := c.ConnectGRPC(ctx, options...)
	if err != nil {
		return nil, err
	}
	stream, err := NewWorkerTunnelClient(conn).OpenTunnel(ctx)
	if err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("tikeo open tunnel failed: %w", err)
	}
	if err := stream.Send(c.registerMessage()); err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("tikeo worker register send failed: %w", err)
	}
	message, err := stream.Recv()
	if err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("tikeo worker register ack failed: %w", err)
	}
	registered := message.GetRegistered()
	if registered == nil {
		_ = conn.Close()
		return nil, errors.New("tikeo worker expected registration ack")
	}
	sdkLog(LogLevelInfo, "registered worker_id=%s lease_seconds=%d generation=%d", registered.GetWorkerId(), registered.GetLeaseSeconds(), registered.GetGeneration())
	return &Session{
		conn:           conn,
		stream:         stream,
		workerID:       registered.GetWorkerId(),
		leaseSeconds:   registered.GetLeaseSeconds(),
		generation:     registered.GetGeneration(),
		fencingToken:   registered.GetFencingToken(),
		heartbeatEvery: c.config.HeartbeatEvery,
	}, nil
}

func (s *Session) WorkerID() string     { return s.workerID }
func (s *Session) LeaseSeconds() uint64 { return s.leaseSeconds }
func (s *Session) Generation() uint64   { return s.generation }

// StartHeartbeat starts a background lease renewal loop for long-lived workers.
// It sends the first heartbeat immediately and then repeats on the configured
// heartbeat interval. The returned function stops the loop and waits for it to
// exit; callers should stop it before closing the session.
func (s *Session) StartHeartbeat(ctx context.Context) func() {
	if ctx == nil {
		ctx = context.Background()
	}
	heartbeatCtx, cancel := context.WithCancel(ctx)
	done := make(chan struct{})
	go func() {
		defer close(done)
		ticker := time.NewTicker(s.heartbeatEvery)
		defer ticker.Stop()
		if _, err := s.SendHeartbeat(); err != nil {
			return
		}
		for {
			select {
			case <-heartbeatCtx.Done():
				return
			case <-ticker.C:
				if _, err := s.SendHeartbeat(); err != nil {
					return
				}
			}
		}
	}()
	return func() {
		cancel()
		<-done
	}
}

// SendHeartbeat renews the worker lease without waiting for the ping response.
// The main receive loop may observe and ignore the ping while waiting for tasks.
func (s *Session) SendHeartbeat() (uint64, error) {
	sequence := atomic.AddUint64(&s.sequence, 1)
	sdkLog(LogLevelDebug, "sending heartbeat worker_id=%s sequence=%d", s.workerID, sequence)
	if err := s.sendWorkerMessage(s.heartbeatMessage(sequence)); err != nil {
		return sequence, err
	}
	return sequence, nil
}

func (s *Session) Heartbeat() (*workerpb.Ping, error) {
	sequence := atomic.AddUint64(&s.sequence, 1)
	if err := s.sendWorkerMessage(s.heartbeatMessage(sequence)); err != nil {
		return nil, err
	}
	for {
		message, err := s.stream.Recv()
		if err != nil {
			return nil, err
		}
		if ping := message.GetPing(); ping != nil && ping.GetSequence() == sequence {
			return ping, nil
		}
	}
}

// EmitTaskLog appends one task log line to the current job instance.
func (s *Session) EmitTaskLog(instanceID, assignmentToken, level, message string) (int64, error) {
	if strings.TrimSpace(level) == "" {
		level = "info"
	}
	sequence := atomic.AddInt64(&s.logSequence, 1)
	err := s.sendWorkerMessage(&workerpb.WorkerMessage{Kind: &workerpb.WorkerMessage_TaskLog{TaskLog: &workerpb.TaskLog{
		WorkerId:        s.workerID,
		InstanceId:      instanceID,
		Level:           level,
		Message:         message,
		Sequence:        sequence,
		AssignmentToken: assignmentToken,
	}}})
	if err != nil {
		return sequence, err
	}
	return sequence, nil
}

func (s *Session) ProcessNext(ctx context.Context, processor TaskProcessor) (TaskOutcome, error) {
	return s.ProcessNextWithScriptRunners(ctx, processor, nil)
}

func (s *Session) ProcessNextWithScriptRunners(ctx context.Context, processor TaskProcessor, scripts *ScriptRunnerRegistry) (TaskOutcome, error) {
	if processor == nil {
		return TaskOutcome{}, errors.New("tikeo task processor is required")
	}
	for {
		message, err := s.stream.Recv()
		if err != nil {
			return TaskOutcome{}, err
		}
		task := message.GetDispatchTask()
		if task == nil {
			continue
		}
		s.emitTaskLogSafely(
			task,
			"info",
			fmt.Sprintf("received task %s processor=%s", task.GetInstanceId(), task.GetProcessorName()),
		)
		outcome, err := processDispatchTaskWithLogs(ctx, processor, scripts, task, func(level, message string) {
			s.emitTaskLogSafely(task, level, message)
		})
		if err != nil {
			outcome = Failed(err.Error())
		}
		level := "info"
		if !outcome.Success {
			level = "error"
		}
		s.emitTaskLogSafely(
			task,
			level,
			fmt.Sprintf("completed task %s success=%t message=%s", task.GetInstanceId(), outcome.Success, outcome.Message),
		)
		if err := s.sendWorkerMessage(&workerpb.WorkerMessage{Kind: &workerpb.WorkerMessage_TaskResult{TaskResult: &workerpb.TaskResult{
			WorkerId:        s.workerID,
			InstanceId:      task.GetInstanceId(),
			Success:         outcome.Success,
			Message:         outcome.Message,
			AssignmentToken: task.GetAssignmentToken(),
		}}}); err != nil {
			return outcome, err
		}
		return outcome, nil
	}
}

func (s *Session) emitTaskLogSafely(task *workerpb.DispatchTask, level, message string) {
	printTaskLogLocally(level, message)
	if _, err := s.EmitTaskLog(task.GetInstanceId(), task.GetAssignmentToken(), level, message); err != nil {
		sdkLog(LogLevelWarning, "failed to emit task log instance_id=%s error=%v", task.GetInstanceId(), err)
	}
}

func printTaskLogLocally(level, message string) {
	line := "[tikeo-worker] " + message
	if strings.EqualFold(level, "error") {
		fmt.Fprintln(os.Stderr, line)
		return
	}
	fmt.Fprintln(os.Stdout, line)
}

func (s *Session) Close() error {
	err := s.sendWorkerMessage(&workerpb.WorkerMessage{Kind: &workerpb.WorkerMessage_Unregister{Unregister: &workerpb.UnregisterWorker{
		WorkerId:     s.workerID,
		Generation:   s.generation,
		FencingToken: s.fencingToken,
	}}})
	closeErr := s.stream.CloseSend()
	connErr := s.conn.Close()
	if err != nil {
		return err
	}
	if closeErr != nil {
		return closeErr
	}
	return connErr
}

func (s *Session) sendWorkerMessage(message *workerpb.WorkerMessage) error {
	s.sendMu.Lock()
	defer s.sendMu.Unlock()
	return s.stream.Send(message)
}

func (s *Session) heartbeatMessage(sequence uint64) *workerpb.WorkerMessage {
	return &workerpb.WorkerMessage{Kind: &workerpb.WorkerMessage_Heartbeat{Heartbeat: &workerpb.Heartbeat{
		WorkerId:     s.workerID,
		Sequence:     sequence,
		Generation:   s.generation,
		FencingToken: s.fencingToken,
	}}}
}

func (c *Client) registerMessage() *workerpb.WorkerMessage {
	return &workerpb.WorkerMessage{Kind: &workerpb.WorkerMessage_Register{Register: &workerpb.RegisterWorker{
		ClientInstanceId:       c.config.ClientInstanceID,
		App:                    c.config.App,
		Namespace:              c.config.Namespace,
		Cluster:                c.config.Cluster,
		Region:                 c.config.Region,
		Capabilities:           append([]string(nil), c.config.Capabilities...),
		Labels:                 cloneMap(c.config.Labels),
		StructuredCapabilities: toProtoCapabilities(c.config.Structured),
		Election: &workerpb.WorkerClusterElection{
			Enabled:  true,
			Priority: 100,
		},
	}}}
}

func toProtoCapabilities(capabilities WorkerCapabilities) *workerpb.WorkerCapabilities {
	out := &workerpb.WorkerCapabilities{
		Tags: append([]string(nil), capabilities.Tags...),
	}
	for _, processor := range capabilities.NormalProcessors {
		if processor.Name == "" {
			continue
		}
		out.NormalProcessors = append(out.NormalProcessors, &workerpb.ProcessorCapability{Name: processor.Name, Description: processor.Description})
	}
	for _, runner := range capabilities.ScriptRunners {
		out.ScriptRunners = append(out.ScriptRunners, &workerpb.ScriptRunnerCapability{
			Language:       runner.Language,
			SandboxBackend: runner.SandboxBackend,
		})
	}
	for _, plugin := range capabilities.PluginProcessors {
		protoPlugin := &workerpb.PluginProcessorCapability{
			Type:           string(plugin.Type),
			ProcessorNames: append([]string(nil), plugin.ProcessorNames...),
		}
		for _, processor := range plugin.Processors {
			protoPlugin.Processors = append(protoPlugin.Processors, &workerpb.ProcessorCapability{Name: processor.Name, Description: processor.Description})
		}
		out.PluginProcessors = append(out.PluginProcessors, protoPlugin)
	}
	return out
}

func processDispatchTask(ctx context.Context, processor TaskProcessor, scripts *ScriptRunnerRegistry, task *workerpb.DispatchTask) (TaskOutcome, error) {
	return processDispatchTaskWithLogs(ctx, processor, scripts, task, nil)
}

func processDispatchTaskWithLogs(ctx context.Context, processor TaskProcessor, scripts *ScriptRunnerRegistry, task *workerpb.DispatchTask, emitLog func(level, message string)) (outcome TaskOutcome, err error) {
	defer func() {
		if recovered := recover(); recovered != nil {
			message := fmt.Sprintf("processor panic: %v", recovered)
			if emitLog != nil {
				emitLog("error", message+"\n"+string(debug.Stack()))
			}
			outcome = Failed(message)
			err = nil
		}
	}()
	if script := task.GetProcessorBinding().GetScript(); script != nil {
		runner := scripts.get(script.GetLanguage())
		if runner == nil {
			sdkLog(LogLevelWarning, "missing script runner language=%s", script.GetLanguage())
			return Failed(fmt.Sprintf("%s script runner is not registered on this worker", script.GetLanguage())), nil
		}
		return runner.Run(ctx, scriptRunnerTask(task, script).withLogSink(emitLog))
	}
	taskContext := TaskContext{
		InstanceID:    task.GetInstanceId(),
		JobID:         task.GetJobId(),
		ProcessorName: task.GetProcessorName(),
		Payload:       task.GetPayload(),
		Log:           emitLog,
	}
	taskCtx := ContextWithTaskLogScope(ctx, TaskLogScope{
		InstanceID:    taskContext.InstanceID,
		JobID:         taskContext.JobID,
		ProcessorName: taskContext.ProcessorName,
		Log:           emitLog,
	})
	outcome, err = processor.Process(taskCtx, taskContext)
	if err != nil && emitLog != nil {
		emitLog("error", "processor error: "+err.Error()+"\n"+string(debug.Stack()))
	}
	return outcome, err
}
