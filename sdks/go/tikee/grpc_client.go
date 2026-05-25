package tikee

import (
	"context"
	"errors"
	"fmt"
	"net/url"
	"strings"

	"github.com/yhyzgn/tikee/sdks/go/tikee/internal/workerpb"
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
	if err := c.config.Validate(); err != nil {
		return nil, err
	}
	if ctx == nil {
		return nil, errors.New("tikee grpc dial context is required")
	}
	if err := ctx.Err(); err != nil {
		return nil, fmt.Errorf("tikee grpc dial context is not usable: %w", err)
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
		return nil, fmt.Errorf("tikee grpc client create failed: %w", err)
	}
	conn.Connect()
	return conn, nil
}

func grpcTarget(endpoint string) (string, error) {
	value := strings.TrimSpace(endpoint)
	if value == "" {
		return "", errors.New("tikee worker endpoint is required")
	}

	parsed, err := url.Parse(value)
	if err != nil {
		if strings.Contains(value, "://") {
			return "", fmt.Errorf("tikee worker endpoint is invalid: %w", err)
		}
		return value, nil
	}
	if parsed.Scheme == "http" || parsed.Scheme == "https" {
		if parsed.Host == "" {
			return "", errors.New("tikee worker endpoint host is required")
		}
		return parsed.Host, nil
	}

	return value, nil
}

// NewWorkerTunnelClient returns the generated Worker Tunnel gRPC client for a connection.
func NewWorkerTunnelClient(conn grpc.ClientConnInterface) WorkerTunnelClient {
	return workerpb.NewWorkerTunnelServiceClient(conn)
}
