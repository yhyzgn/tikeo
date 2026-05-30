package controller

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

const (
	GroupVersionString = "tikee.io/v1alpha1"
	Kind               = "TikeeManifest"
	DefaultApplyMode   = "diffOnly"
)

type TikeeManifest struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`
	Spec              TikeeManifestSpec   `json:"spec,omitempty"`
	Status            TikeeManifestStatus `json:"status,omitempty"`
}

type TikeeManifestList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []TikeeManifest `json:"items"`
}

type TikeeManifestSpec struct {
	Manifest          runtime.RawExtension `json:"manifest"`
	ApplyMode         string               `json:"applyMode,omitempty"`
	TikeeEndpointRef  *KeyRef              `json:"tikeeEndpointRef,omitempty"`
	APITokenSecretRef *KeyRef              `json:"apiTokenSecretRef,omitempty"`
}

type KeyRef struct {
	ConfigMapName string `json:"configMapName,omitempty"`
	SecretName    string `json:"secretName,omitempty"`
	Key           string `json:"key,omitempty"`
}

type TikeeManifestStatus struct {
	ObservedGeneration int64                  `json:"observedGeneration,omitempty"`
	Checksum           string                 `json:"checksum,omitempty"`
	CurrentChecksum    string                 `json:"currentChecksum,omitempty"`
	DesiredChecksum    string                 `json:"desiredChecksum,omitempty"`
	Summary            map[string]uint64      `json:"summary,omitempty"`
	LastDiff           []map[string]any       `json:"lastDiff,omitempty"`
	Conditions         []metav1.Condition     `json:"conditions,omitempty"`
}
