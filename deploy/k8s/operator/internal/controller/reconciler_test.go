package controller

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

func TestReconcilerDiffsManifestAndBuildsStatus(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != GitOpsDiffPath { t.Fatalf("unexpected path %s", r.URL.Path) }
		if r.Header.Get("Authorization") != "Bearer token" { t.Fatalf("missing authorization") }
		_, _ = w.Write([]byte(`{"code":0,"data":{"currentChecksum":"sha256:old","desiredChecksum":"sha256:new","summary":{"unchanged":1},"changes":[{"action":"unchanged","kind":"Job","name":"demo"}]}}`))
	}))
	defer server.Close()
	manifest := &TikeoManifest{ObjectMeta: metav1.ObjectMeta{Name: "demo", Generation: 7}, Spec: TikeoManifestSpec{ApplyMode: "diffOnly", Manifest: runtime.RawExtension{Raw: []byte(`{"apiVersion":"tikeo.io/v1alpha1","kind":"TikeoManifest","scope":{},"resources":[]}`)}}}
	result, err := Reconciler{Client: TikeoClient{Endpoint: server.URL, Token: "token"}}.Reconcile(context.Background(), manifest)
	if err != nil { t.Fatal(err) }
	if result.Status.ObservedGeneration != 7 || result.Status.DesiredChecksum != "sha256:new" || result.Status.Summary["unchanged"] != 1 { t.Fatalf("unexpected status: %+v", result.Status) }
	if len(result.Status.Conditions) != 1 || result.Status.Conditions[0].Reason != "DiffOnly" { t.Fatalf("unexpected conditions: %+v", result.Status.Conditions) }
}

func TestReconcilerFailsClosedForBadApplyMode(t *testing.T) {
	manifest := &TikeoManifest{Spec: TikeoManifestSpec{ApplyMode: "force", Manifest: runtime.RawExtension{Raw: []byte(`{}`)}}}
	if _, err := (Reconciler{}).Reconcile(context.Background(), manifest); err == nil { t.Fatal("expected applyMode error") }
}
