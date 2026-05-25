package tikee

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
	Capabilities     []string
	HeartbeatEvery   time.Duration
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
		HeartbeatEvery:   10 * time.Second,
	}
}

// Validate checks fields before a future gRPC session dials the server.
func (c WorkerConfig) Validate() error {
	if strings.TrimSpace(c.Endpoint) == "" {
		return errors.New("tikee worker endpoint is required")
	}
	if strings.TrimSpace(c.ClientInstanceID) == "" {
		return errors.New("tikee client instance id is required")
	}
	if strings.TrimSpace(c.Namespace) == "" {
		return errors.New("tikee worker namespace is required")
	}
	if strings.TrimSpace(c.App) == "" {
		return errors.New("tikee worker app is required")
	}
	if strings.TrimSpace(c.Name) == "" {
		return errors.New("tikee worker name is required")
	}
	if c.HeartbeatEvery <= 0 {
		return errors.New("tikee heartbeat interval must be positive")
	}
	return nil
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
