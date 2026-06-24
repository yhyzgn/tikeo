package tikeo

import (
	"context"
	"errors"
	"sync"
	"time"
)

// Registration is the protocol-neutral worker registration snapshot.
type Registration struct {
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
}

// Heartbeat is the protocol-neutral heartbeat snapshot.
type Heartbeat struct {
	WorkerID     string
	Sequence     uint64
	Generation   uint64
	FencingToken string
	SentAt       time.Time
}

// Client is the first Go Worker SDK boundary. It validates config, exposes
// dry-run heartbeat helpers, and can create official generated gRPC clients.
type Client struct {
	config WorkerConfig
	mu     sync.Mutex
	seq    uint64
	open   bool
}

// NewClient constructs a Worker client.
func NewClient(config WorkerConfig) (*Client, error) {
	if err := config.Validate(); err != nil {
		return nil, err
	}
	config.Capabilities = normalizedCapabilities(config.Capabilities)
	config.Structured.Tags = normalizedCapabilities(config.Structured.Tags)
	config.Structured.NormalProcessors = normalizeProcessors(config.Structured.NormalProcessors)
	for i := range config.Structured.PluginProcessors {
		config.Structured.PluginProcessors[i].ProcessorNames = normalizedCapabilities(config.Structured.PluginProcessors[i].ProcessorNames)
		config.Structured.PluginProcessors[i].Processors = normalizeProcessors(config.Structured.PluginProcessors[i].Processors)
		for _, name := range config.Structured.PluginProcessors[i].ProcessorNames {
			config.Structured.PluginProcessors[i].Processors = appendUniqueProcessor(config.Structured.PluginProcessors[i].Processors, newProcessorCapability(name))
		}
		config.Structured.PluginProcessors[i].ProcessorNames = nil
		for _, processor := range config.Structured.PluginProcessors[i].Processors {
			config.Structured.PluginProcessors[i].ProcessorNames = append(config.Structured.PluginProcessors[i].ProcessorNames, processor.Name)
		}
	}
	if config.Labels == nil {
		config.Labels = map[string]string{}
	}
	return &Client{config: config}, nil
}

// Registration returns the protocol-neutral registration snapshot.
func (c *Client) Registration() Registration {
	return Registration{
		ClientInstanceID: c.config.ClientInstanceID,
		Namespace:        c.config.Namespace,
		App:              c.config.App,
		Name:             c.config.Name,
		Region:           c.config.Region,
		Version:          c.config.Version,
		Cluster:          c.config.Cluster,
		Capabilities:     append([]string(nil), c.config.Capabilities...),
		Labels:           cloneMap(c.config.Labels),
		Structured:       cloneWorkerCapabilities(c.config.Structured),
	}
}

// StartDryRun validates the processor and marks the client ready for heartbeat
// generation. It intentionally does not dial the network yet.
func (c *Client) StartDryRun(_ context.Context, processor TaskProcessor) error {
	if processor == nil {
		return errors.New("tikeo task processor is required")
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	c.open = true
	return nil
}

func cloneMap(values map[string]string) map[string]string {
	out := make(map[string]string, len(values))
	for key, value := range values {
		out[key] = value
	}
	return out
}

func cloneWorkerCapabilities(in WorkerCapabilities) WorkerCapabilities {
	out := WorkerCapabilities{
		Tags:             append([]string(nil), in.Tags...),
		NormalProcessors: append([]ProcessorCapability(nil), in.NormalProcessors...),
	}
	for _, runner := range in.ScriptRunners {
		out.ScriptRunners = append(out.ScriptRunners, runner)
	}
	for _, plugin := range in.PluginProcessors {
		out.PluginProcessors = append(out.PluginProcessors, PluginProcessorCapability{
			Type:           plugin.Type,
			Processors:     append([]ProcessorCapability(nil), plugin.Processors...),
			ProcessorNames: append([]string(nil), plugin.ProcessorNames...),
		})
	}
	return out
}

// NextHeartbeat returns the next local heartbeat shape.
func (c *Client) NextHeartbeat(workerID, fencingToken string, generation uint64) (Heartbeat, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if !c.open {
		return Heartbeat{}, errors.New("tikeo worker client is not started")
	}
	if workerID == "" {
		return Heartbeat{}, errors.New("tikeo worker id is required")
	}
	c.seq++
	return Heartbeat{
		WorkerID:     workerID,
		Sequence:     c.seq,
		Generation:   generation,
		FencingToken: fencingToken,
		SentAt:       time.Now().UTC(),
	}, nil
}

// Close stops local dry-run heartbeat generation.
func (c *Client) Close() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.open = false
}
