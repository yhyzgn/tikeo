package tikee

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
	Capabilities     []string
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
		Capabilities:     append([]string(nil), c.config.Capabilities...),
	}
}

// StartDryRun validates the processor and marks the client ready for heartbeat
// generation. It intentionally does not dial the network yet.
func (c *Client) StartDryRun(_ context.Context, processor TaskProcessor) error {
	if processor == nil {
		return errors.New("tikee task processor is required")
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	c.open = true
	return nil
}

// NextHeartbeat returns the next local heartbeat shape.
func (c *Client) NextHeartbeat(workerID, fencingToken string, generation uint64) (Heartbeat, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if !c.open {
		return Heartbeat{}, errors.New("tikee worker client is not started")
	}
	if workerID == "" {
		return Heartbeat{}, errors.New("tikee worker id is required")
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
