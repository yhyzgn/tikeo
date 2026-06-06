package controller

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"time"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// Reconciler calls /api/v1/gitops/diff and projects the review result into CRD status.
type Reconciler struct { Client TikeoClient }

type ReconcileResult struct { Status TikeoManifestStatus }

func (r Reconciler) Reconcile(ctx context.Context, resource *TikeoManifest) (*ReconcileResult, error) {
	if resource == nil { return nil, fmt.Errorf("TikeoManifest is required") }
	mode := resource.Spec.ApplyMode
	if mode == "" { mode = DefaultApplyMode }
	if mode != "diffOnly" && mode != "apply" { return nil, fmt.Errorf("spec.applyMode must be diffOnly or apply") }
	manifest := resource.Spec.Manifest.Raw
	if len(manifest) == 0 { return nil, fmt.Errorf("spec.manifest is required") }
	diff, err := r.Client.Diff(ctx, manifest)
	status := TikeoManifestStatus{ObservedGeneration: resource.Generation, Checksum: checksum(manifest)}
	if err != nil {
		status.Conditions = []metav1.Condition{condition("Ready", metav1.ConditionFalse, "DiffFailed", err.Error())}
		return &ReconcileResult{Status: status}, err
	}
	status.CurrentChecksum = diff.CurrentChecksum
	status.DesiredChecksum = diff.DesiredChecksum
	status.Summary = diff.Summary
	status.LastDiff = diff.Changes
	if mode == "apply" {
		status.Conditions = []metav1.Condition{condition("Ready", metav1.ConditionTrue, "DiffReviewed", "applyMode=apply is accepted by the operator, but mutations remain delegated to typed tikeo CRUD APIs")}
	} else {
		status.Conditions = []metav1.Condition{condition("Ready", metav1.ConditionTrue, "DiffOnly", "diffOnly completed without applying mutations")}
	}
	return &ReconcileResult{Status: status}, nil
}

func checksum(payload []byte) string {
	sum := sha256.Sum256(payload)
	return "sha256:" + hex.EncodeToString(sum[:])
}

func condition(kind string, status metav1.ConditionStatus, reason, message string) metav1.Condition {
	return metav1.Condition{Type: kind, Status: status, Reason: reason, Message: message, LastTransitionTime: metav1.NewTime(time.Now().UTC())}
}
