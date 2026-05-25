package tikee

import (
	"context"
	"strings"
	"testing"
)

func TestClientRegistrationAndHeartbeatDryRun(t *testing.T) {
	config := LocalConfig("http://127.0.0.1:9998", "go-worker-1")
	config.Namespace = "tenant-a"
	config.App = "billing"
	config.Capabilities = []string{"echo", "echo", " script:shell ", ""}
	client, err := NewClient(config)
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}

	registration := client.Registration()
	if registration.ClientInstanceID != "go-worker-1" || registration.Namespace != "tenant-a" || registration.App != "billing" {
		t.Fatalf("unexpected registration: %+v", registration)
	}
	if got, want := strings.Join(registration.Capabilities, ","), "echo,script:shell"; got != want {
		t.Fatalf("capabilities = %q, want %q", got, want)
	}

	processor := TaskProcessorFunc(func(context.Context, TaskContext) (TaskOutcome, error) {
		return Succeeded(), nil
	})
	if err := client.StartDryRun(context.Background(), processor); err != nil {
		t.Fatalf("StartDryRun() error = %v", err)
	}
	heartbeat, err := client.NextHeartbeat("worker-1", "fence-1", 3)
	if err != nil {
		t.Fatalf("NextHeartbeat() error = %v", err)
	}
	if heartbeat.Sequence != 1 || heartbeat.Generation != 3 || heartbeat.FencingToken != "fence-1" {
		t.Fatalf("unexpected heartbeat: %+v", heartbeat)
	}
}

func TestConfigValidationFailsClosed(t *testing.T) {
	_, err := NewClient(WorkerConfig{})
	if err == nil || !strings.Contains(err.Error(), "endpoint") {
		t.Fatalf("expected endpoint validation error, got %v", err)
	}

	config := LocalConfig("http://127.0.0.1:9998", "go-worker-2")
	config.HeartbeatEvery = 0
	_, err = NewClient(config)
	if err == nil || !strings.Contains(err.Error(), "heartbeat") {
		t.Fatalf("expected heartbeat validation error, got %v", err)
	}
}

func TestGRPCTargetNormalizesHTTPURLs(t *testing.T) {
	cases := map[string]string{
		"127.0.0.1:9998":             "127.0.0.1:9998",
		" http://127.0.0.1:9998 ":    "127.0.0.1:9998",
		"https://worker.example:443": "worker.example:443",
		"dns:///worker.example:443":  "dns:///worker.example:443",
	}
	for endpoint, want := range cases {
		got, err := grpcTarget(endpoint)
		if err != nil {
			t.Fatalf("grpcTarget(%q) error = %v", endpoint, err)
		}
		if got != want {
			t.Fatalf("grpcTarget(%q) = %q, want %q", endpoint, got, want)
		}
	}
}

func TestConnectGRPCUsesOfficialClientBoundary(t *testing.T) {
	client, err := NewClient(LocalConfig("http://127.0.0.1:9998", "go-worker-grpc"))
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}

	if _, err := client.ConnectGRPC(nil); err == nil || !strings.Contains(err.Error(), "context") {
		t.Fatalf("expected nil context error, got %v", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if _, err := client.ConnectGRPC(ctx); err == nil || !strings.Contains(err.Error(), "context") {
		t.Fatalf("expected canceled context error, got %v", err)
	}

	conn, err := client.ConnectGRPC(context.Background())
	if err != nil {
		t.Fatalf("ConnectGRPC() error = %v", err)
	}
	if err := conn.Close(); err != nil {
		t.Fatalf("ClientConn.Close() error = %v", err)
	}
}

func TestGeneratedWorkerTunnelClientCanBeConstructed(t *testing.T) {
	client, err := NewClient(LocalConfig("127.0.0.1:9998", "go-worker-generated"))
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}
	conn, err := client.ConnectGRPC(context.Background())
	if err != nil {
		t.Fatalf("ConnectGRPC() error = %v", err)
	}
	defer conn.Close()
	if generated := NewWorkerTunnelClient(conn); generated == nil {
		t.Fatal("NewWorkerTunnelClient() returned nil")
	}
}
