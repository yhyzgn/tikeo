package tikee

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const (
	ManifestPath = "/api/v1/gitops/manifest"
	DiffPath     = "/api/v1/gitops/diff"
)

type Client struct {
	baseURL    string
	apiToken   string
	httpClient *http.Client
}

type Config struct {
	Endpoint string
	APIToken string
	Timeout  time.Duration
}

func NewClient(config Config) (*Client, error) {
	endpoint := strings.TrimRight(strings.TrimSpace(config.Endpoint), "/")
	if endpoint == "" {
		return nil, fmt.Errorf("tikee endpoint is required")
	}
	parsed, err := url.Parse(endpoint)
	if err != nil || parsed.Scheme == "" || parsed.Host == "" {
		return nil, fmt.Errorf("tikee endpoint must be an absolute URL")
	}
	if strings.TrimSpace(config.APIToken) == "" {
		return nil, fmt.Errorf("tikee api token is required")
	}
	timeout := config.Timeout
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	return &Client{baseURL: endpoint, apiToken: config.APIToken, httpClient: &http.Client{Timeout: timeout}}, nil
}

func (c *Client) ExportManifest(ctx context.Context, namespace, app, format string) (json.RawMessage, error) {
	values := url.Values{}
	if namespace != "" {
		values.Set("namespace", namespace)
	}
	if app != "" {
		values.Set("app", app)
	}
	if format != "" {
		values.Set("format", format)
	}
	path := ManifestPath
	if encoded := values.Encode(); encoded != "" {
		path += "?" + encoded
	}
	return c.do(ctx, http.MethodGet, path, nil)
}

func (c *Client) DiffManifest(ctx context.Context, manifest json.RawMessage) (json.RawMessage, error) {
	if len(bytes.TrimSpace(manifest)) == 0 {
		return nil, fmt.Errorf("manifest JSON is required")
	}
	var candidate map[string]any
	if err := json.Unmarshal(manifest, &candidate); err != nil {
		return nil, fmt.Errorf("manifest must be valid JSON: %w", err)
	}
	body, err := json.Marshal(map[string]json.RawMessage{"manifest": manifest})
	if err != nil {
		return nil, err
	}
	return c.do(ctx, http.MethodPost, DiffPath, body)
}

func (c *Client) do(ctx context.Context, method, path string, body []byte) (json.RawMessage, error) {
	request, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Accept", "application/json")
	request.Header.Set("Authorization", "Bearer "+c.apiToken)
	if body != nil {
		request.Header.Set("Content-Type", "application/json")
	}
	response, err := c.httpClient.Do(request)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()
	payload, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, err
	}
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		return nil, fmt.Errorf("tikee API %s %s returned %s: %s", method, path, response.Status, string(payload))
	}
	var envelope struct {
		Code    int             `json:"code"`
		Message string          `json:"message"`
		Data    json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal(payload, &envelope); err != nil {
		return nil, fmt.Errorf("tikee API returned invalid JSON envelope: %w", err)
	}
	if envelope.Code != 0 {
		return nil, fmt.Errorf("tikee API returned code %d: %s", envelope.Code, envelope.Message)
	}
	return envelope.Data, nil
}
