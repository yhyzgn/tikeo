package tikeo

import (
	"errors"
	"strings"
	"time"
)

// WorkerConfig describes one outbound Worker Tunnel client instance.
type WorkerConfig struct {
	Endpoint         string
	ClientInstanceID string
	Namespace        string
	App              string
	Name             string
	Region           string
	Version          string
	Cluster          string
	Capabilities     []string
	Labels           map[string]string
	Structured       WorkerCapabilities
	HeartbeatEvery   time.Duration
}

type WorkerCapabilities struct {
	Tags             []string
	SDKProcessors    []string
	ScriptRunners    []ScriptRunnerCapability
	PluginProcessors []PluginProcessorCapability
}

type ScriptRunnerCapability struct {
	Language       string
	SandboxBackend string
}

type PluginProcessorCapability struct {
	Type           string
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
