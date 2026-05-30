package controller

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const GitOpsDiffPath = "/api/v1/gitops/diff"

type TikeeClient struct {
	Endpoint string
	Token    string
	HTTP     *http.Client
}

type DiffResponse struct {
	CurrentChecksum string           `json:"currentChecksum"`
	DesiredChecksum string           `json:"desiredChecksum"`
	Summary         map[string]uint64 `json:"summary"`
	Changes         []map[string]any  `json:"changes"`
}

func (c TikeeClient) Diff(ctx context.Context, manifest []byte) (*DiffResponse, error) {
	endpoint := strings.TrimRight(strings.TrimSpace(c.Endpoint), "/")
	if endpoint == "" { return nil, fmt.Errorf("tikee endpoint is required") }
	if strings.TrimSpace(c.Token) == "" { return nil, fmt.Errorf("tikee api token is required") }
	var candidate map[string]any
	if err := json.Unmarshal(manifest, &candidate); err != nil { return nil, fmt.Errorf("spec.manifest must be JSON object: %w", err) }
	body, err := json.Marshal(map[string]json.RawMessage{"manifest": manifest})
	if err != nil { return nil, err }
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, endpoint+GitOpsDiffPath, bytes.NewReader(body))
	if err != nil { return nil, err }
	request.Header.Set("Authorization", "Bearer "+c.Token)
	request.Header.Set("Content-Type", "application/json")
	client := c.HTTP
	if client == nil { client = &http.Client{Timeout: 30 * time.Second} }
	response, err := client.Do(request)
	if err != nil { return nil, err }
	defer response.Body.Close()
	payload, err := io.ReadAll(response.Body)
	if err != nil { return nil, err }
	if response.StatusCode < 200 || response.StatusCode >= 300 { return nil, fmt.Errorf("tikee diff returned %s: %s", response.Status, string(payload)) }
	var envelope struct {
		Code int `json:"code"`
		Message string `json:"message"`
		Data DiffResponse `json:"data"`
	}
	if err := json.Unmarshal(payload, &envelope); err != nil { return nil, err }
	if envelope.Code != 0 { return nil, fmt.Errorf("tikee diff returned code %d: %s", envelope.Code, envelope.Message) }
	return &envelope.Data, nil
}
