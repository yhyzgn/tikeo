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

// ProcessorCapability declares one normal or plugin processor and optional display metadata.
type ProcessorCapability struct {
	// Name is the stable processorName used by job definitions.
	Name string
	// Description is optional operator-facing help text.
	Description string
}

// PluginType is a constrained plugin processor type value.
type PluginType string

const (
	// PluginTypeSQL declares a SQL-oriented plugin processor.
	PluginTypeSQL PluginType = "sql"
	// PluginTypeHTTP declares an HTTP/API plugin processor.
	PluginTypeHTTP PluginType = "http"
	// PluginTypeNotification declares a notification plugin processor.
	PluginTypeNotification PluginType = "notification"
	// PluginTypeCustom is an explicit extension point for project-specific plugin types.
	PluginTypeCustom PluginType = "custom"
)

// Valid reports whether the plugin type is one of the constrained values accepted by tikeo.
func (t PluginType) Valid() bool {
	switch t {
	case PluginTypeSQL, PluginTypeHTTP, PluginTypeNotification, PluginTypeCustom:
		return true
	default:
		return false
	}
}

// WorkerCapabilities contains typed routing and operator capability declarations.
type WorkerCapabilities struct {
	// Tags are operator-facing structured labels.
	Tags []string
	// NormalProcessors are normal application processor declarations.
	NormalProcessors []ProcessorCapability
	// ScriptRunners are language/backend sandbox declarations.
	ScriptRunners []ScriptRunnerCapability
	// PluginProcessors are plugin type plus concrete processor declarations.
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
	Type PluginType
	// Processors are concrete executor declarations for this plugin type.
	Processors []ProcessorCapability
	// ProcessorNames are legacy concrete executor names for this plugin type. Prefer Processors.
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

func (c *WorkerConfig) AddNormalProcessor(name string, description ...string) {
	processor := newProcessorCapability(name, description...)
	if processor.Name == "" {
		return
	}
	c.Structured.NormalProcessors = appendUniqueProcessor(c.Structured.NormalProcessors, processor)
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

func (c *WorkerConfig) AddPluginProcessor(processorType PluginType, processorName string, description ...string) {
	processorType = PluginType(strings.TrimSpace(string(processorType)))
	processor := newProcessorCapability(processorName, description...)
	if !processorType.Valid() || processor.Name == "" {
		return
	}
	for i := range c.Structured.PluginProcessors {
		if c.Structured.PluginProcessors[i].Type == processorType {
			c.Structured.PluginProcessors[i].Processors = appendUniqueProcessor(c.Structured.PluginProcessors[i].Processors, processor)
			c.Structured.PluginProcessors[i].ProcessorNames = appendUnique(c.Structured.PluginProcessors[i].ProcessorNames, processor.Name)
			return
		}
	}
	c.Structured.PluginProcessors = append(c.Structured.PluginProcessors, PluginProcessorCapability{
		Type:           processorType,
		Processors:     []ProcessorCapability{processor},
		ProcessorNames: []string{processor.Name},
	})
}

func newProcessorCapability(name string, description ...string) ProcessorCapability {
	processor := ProcessorCapability{Name: strings.TrimSpace(name)}
	if len(description) > 0 {
		processor.Description = strings.TrimSpace(description[0])
	}
	return processor
}

func appendUniqueProcessor(values []ProcessorCapability, value ProcessorCapability) []ProcessorCapability {
	if value.Name == "" {
		return values
	}
	for i := range values {
		if values[i].Name == value.Name {
			if values[i].Description == "" && value.Description != "" {
				values[i].Description = value.Description
			}
			return values
		}
	}
	return append(values, value)
}

// Validate checks fields before a future gRPC session dials the server.
func normalizeProcessors(values []ProcessorCapability) []ProcessorCapability {
	out := []ProcessorCapability{}
	for _, value := range values {
		out = appendUniqueProcessor(out, newProcessorCapability(value.Name, value.Description))
	}
	return out
}

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
