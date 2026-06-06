package tikeo

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestClientExportsManifestAndPostsDiffWithAuthorization(t *testing.T) {
	var sawAuthorization bool
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		sawAuthorization = r.Header.Get("Authorization") == "Bearer token-1"
		switch r.URL.Path {
		case ManifestPath:
			if r.URL.Query().Get("namespace") != "default" || r.URL.Query().Get("format") != "yaml" { t.Fatalf("unexpected manifest query: %s", r.URL.RawQuery) }
			_, _ = w.Write([]byte(`{"code":0,"data":{"manifest":{"apiVersion":"tikeo.io/v1alpha1","kind":"TikeoManifest","scope":{},"resources":[]},"manifestYaml":"apiVersion: tikeo.io/v1alpha1","checksum":"sha256:abc"}}`))
		case DiffPath:
			var request map[string]json.RawMessage
			if err := json.NewDecoder(r.Body).Decode(&request); err != nil { t.Fatal(err) }
			if len(request["manifest"]) == 0 { t.Fatal("manifest request missing") }
			_, _ = w.Write([]byte(`{"code":0,"data":{"currentChecksum":"sha256:old","desiredChecksum":"sha256:new","summary":{"update":1},"changes":[]}}`))
		default:
			t.Fatalf("unexpected path %s", r.URL.Path)
		}
	}))
	defer server.Close()
	client, err := NewClient(Config{Endpoint: server.URL, APIToken: "token-1"})
	if err != nil { t.Fatal(err) }
	if _, err := client.ExportManifest(context.Background(), "default", "", "yaml"); err != nil { t.Fatal(err) }
	if _, err := client.DiffManifest(context.Background(), json.RawMessage(`{"apiVersion":"tikeo.io/v1alpha1","kind":"TikeoManifest","scope":{},"resources":[]}`)); err != nil { t.Fatal(err) }
	if !sawAuthorization { t.Fatal("Authorization header was not sent") }
}

func TestClientFailsClosedOnInvalidConfigAndManifest(t *testing.T) {
	if _, err := NewClient(Config{}); err == nil { t.Fatal("expected endpoint error") }
	client, err := NewClient(Config{Endpoint: "https://tikeo.example", APIToken: "token"})
	if err != nil { t.Fatal(err) }
	if _, err := client.DiffManifest(context.Background(), json.RawMessage(`not-json`)); err == nil { t.Fatal("expected invalid manifest error") }
}
