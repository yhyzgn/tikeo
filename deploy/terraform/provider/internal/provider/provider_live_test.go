package provider

import (
	"context"
	"encoding/json"
	"os"
	"testing"

	"github.com/yhyzgn/tikeo/deploy/terraform/provider/internal/tikeo"
)

func TestLiveProviderDriftReview(t *testing.T) {
	endpoint := os.Getenv("TIKEO_TEST_HTTP_URL")
	apiToken := os.Getenv("TIKEO_TEST_API_TOKEN")
	if endpoint == "" || apiToken == "" {
		t.Skip("TIKEO_TEST_HTTP_URL and TIKEO_TEST_API_TOKEN are required for live provider test")
	}

	client, err := tikeo.NewClient(tikeo.Config{
		Endpoint: endpoint,
		APIToken: apiToken,
	})
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}

	// 1. Export manifest
	manifestJSON, err := client.ExportManifest(context.Background(), "default", "billing", "json")
	if err != nil {
		t.Fatalf("failed to export manifest: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(manifestJSON, &parsed); err != nil {
		t.Fatalf("exported manifest is not valid json: %v", err)
	}

	// Read from envelope directly or via "data"
	dataMap := parsed
	if val, exists := parsed["data"]; exists {
		if dm, ok := val.(map[string]interface{}); ok {
			dataMap = dm
		}
	}

	manifestObj, ok := dataMap["manifest"].(map[string]interface{})
	if !ok {
		t.Fatalf("manifest field not found or not map: %v", dataMap)
	}

	// 2. Diff manifest
	manifestRaw, err := json.Marshal(manifestObj)
	if err != nil {
		t.Fatalf("failed to marshal inner manifest: %v", err)
	}

	diffPayload, err := client.DiffManifest(context.Background(), json.RawMessage(manifestRaw))
	if err != nil {
		t.Fatalf("failed to diff manifest: %v", err)
	}

	var diffResult struct {
		CurrentChecksum string `json:"currentChecksum"`
		DesiredChecksum string `json:"desiredChecksum"`
	}
	// The API returns envelope too, so handle both direct and inner data
	var envelope struct {
		Code int             `json:"code"`
		Data json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal(diffPayload, &envelope); err == nil && envelope.Code == 0 && len(envelope.Data) > 0 {
		diffPayload = envelope.Data
	}

	if err := json.Unmarshal(diffPayload, &diffResult); err != nil {
		t.Fatalf("failed to unmarshal diff result: %v", err)
	}

	if diffResult.CurrentChecksum == "" || diffResult.DesiredChecksum == "" {
		t.Fatalf("checksum fields are empty: %s", string(diffPayload))
	}

	t.Logf("Live provider diff verification passed. Current checksum: %s", diffResult.CurrentChecksum)
}
