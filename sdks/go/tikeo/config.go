package tikeo

import (
	"errors"
	"strings"
	"time"
)

// WorkerConfig describes one outbound Worker Tunnel client instance.
type WorkerConfig struct {
	// Endpoint is the Worker Tunnel endpoint, for example http://127.0.0.1:9998.
	Endpoint string
	// ClientInstanceID is a stable client-side hint; Tikeo still assigns worker_id.
	ClientInstanceID string
	// Namespace scopes the worker for dispatch.
	Namespace string
	// App scopes the worker within the namespace.
	App string
	// Name is the operator-facing worker name.
	Name string
	// Region identifies the worker runtime location.
	Region string
	// Version identifies application or worker build version.
	Version string
	// Cluster identifies the worker cluster domain.
	Cluster string
	// Capabilities preserves legacy operator metadata; routing uses Structured.
	Capabilities []string
	// Labels are operator-facing key/value metadata.
	Labels map[string]string
	// Structured declares dispatch-routing capabilities.
	Structured WorkerCapabilities
	// HeartbeatEvery controls worker lease renewal cadence.
	HeartbeatEvery time.Duration
}

// WorkerCapabilities contains typed routing and operator capability declarations.
type WorkerCapabilities struct {
	// Tags are operator-facing structured labels.
	Tags []string
	// SDKProcessors are normal application processor names.
	SDKProcessors []string
	// ScriptRunners are language/backend sandbox declarations.
	ScriptRunners []ScriptRunnerCapability
	// PluginProcessors are plugin type plus concrete processor-name declarations.
	PluginProcessors []PluginProcessorCapability
}

// ScriptRunnerCapability declares one script language and sandbox backend.
type ScriptRunnerCapability struct {
	// Language is the canonical script language name.
	Language string
	// SandboxBackend is auto, srt, deno, wasmtime, wasmedge, v8, docker, podman, or custom.
	SandboxBackend string
}

// PluginProcessorCapability declares plugin dispatch capability.
type PluginProcessorCapability struct {
	// Type is the structured plugin processor type.
	Type string
	// ProcessorNames are concrete executor names for this plugin type.
	ProcessorNames []string
}

// LocalConfig returns a development-friendly worker config.
func LocalConfig(endpoint, clientInstanceID string) WorkerConfig {
	return WorkerConfig{
		Endpoint:         endpoint,
		ClientInstanceID: clientInstanceID,
		Namespace:        "default",
		App:              "default",
		Name:             clientInstanceID,
		Region:           "local",
		Version:          "dev",
		Cluster:          "local",
		Labels:           map[string]string{},
		HeartbeatEvery:   10 * time.Second,
	}
}

func (c *WorkerConfig) AddTag(tag string) {
	c.Structured.Tags = appendUnique(c.Structured.Tags, tag)
}

func (c *WorkerConfig) AddSDKProcessor(name string) {
	c.Structured.SDKProcessors = appendUnique(c.Structured.SDKProcessors, name)
}

func (c *WorkerConfig) AddScriptRunner(language, sandboxBackend string) {
	language = strings.TrimSpace(language)
	if language == "" {
		return
	}
	for _, runner := range c.Structured.ScriptRunners {
		if runner.Language == language {
			return
		}
	}
	c.Structured.ScriptRunners = append(c.Structured.ScriptRunners, ScriptRunnerCapability{
		Language:       language,
		SandboxBackend: strings.TrimSpace(sandboxBackend),
	})
}

func (c *WorkerConfig) AddPluginProcessor(processorType, processorName string) {
	processorType = strings.TrimSpace(processorType)
	processorName = strings.TrimSpace(processorName)
	if processorType == "" || processorName == "" {
		return
	}
	for i := range c.Structured.PluginProcessors {
		if c.Structured.PluginProcessors[i].Type == processorType {
			c.Structured.PluginProcessors[i].ProcessorNames = appendUnique(c.Structured.PluginProcessors[i].ProcessorNames, processorName)
			return
		}
	}
	c.Structured.PluginProcessors = append(c.Structured.PluginProcessors, PluginProcessorCapability{
		Type:           processorType,
		ProcessorNames: []string{processorName},
	})
}

// Validate checks fields before a future gRPC session dials the server.
func (c WorkerConfig) Validate() error {
	if strings.TrimSpace(c.Endpoint) == "" {
		return errors.New("tikeo worker endpoint is required")
	}
	if strings.TrimSpace(c.ClientInstanceID) == "" {
		return errors.New("tikeo client instance id is required")
	}
	if strings.TrimSpace(c.Namespace) == "" {
		return errors.New("tikeo worker namespace is required")
	}
	if strings.TrimSpace(c.App) == "" {
		return errors.New("tikeo worker app is required")
	}
	if strings.TrimSpace(c.Name) == "" {
		return errors.New("tikeo worker name is required")
	}
	if strings.TrimSpace(c.Cluster) == "" {
		return errors.New("tikeo worker cluster is required")
	}
	if c.HeartbeatEvery <= 0 {
		return errors.New("tikeo heartbeat interval must be positive")
	}
	return nil
}

func appendUnique(values []string, value string) []string {
	item := strings.TrimSpace(value)
	if item == "" {
		return values
	}
	for _, existing := range values {
		if existing == item {
			return values
		}
	}
	return append(values, item)
}

func normalizedCapabilities(values []string) []string {
	out := make([]string, 0, len(values))
	seen := map[string]struct{}{}
	for _, value := range values {
		item := strings.TrimSpace(value)
		if item == "" {
			continue
		}
		if _, ok := seen[item]; ok {
			continue
		}
		seen[item] = struct{}{}
		out = append(out, item)
	}
	return out
}
