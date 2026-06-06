package controller

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

const (
	GroupVersionString = "tikeo.io/v1alpha1"
	Kind               = "TikeoManifest"
	DefaultApplyMode   = "diffOnly"
)

type TikeoManifest struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`
	Spec              TikeoManifestSpec   `json:"spec,omitempty"`
	Status            TikeoManifestStatus `json:"status,omitempty"`
}

type TikeoManifestList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []TikeoManifest `json:"items"`
}

type TikeoManifestSpec struct {
	Manifest          runtime.RawExtension `json:"manifest"`
	ApplyMode         string               `json:"applyMode,omitempty"`
	TikeoEndpointRef  *KeyRef              `json:"tikeoEndpointRef,omitempty"`
	APITokenSecretRef *KeyRef              `json:"apiTokenSecretRef,omitempty"`
}

type KeyRef struct {
	ConfigMapName string `json:"configMapName,omitempty"`
	SecretName    string `json:"secretName,omitempty"`
	Key           string `json:"key,omitempty"`
}

type TikeoManifestStatus struct {
	ObservedGeneration int64                  `json:"observedGeneration,omitempty"`
	Checksum           string                 `json:"checksum,omitempty"`
	CurrentChecksum    string                 `json:"currentChecksum,omitempty"`
	DesiredChecksum    string                 `json:"desiredChecksum,omitempty"`
	Summary            map[string]uint64      `json:"summary,omitempty"`
	LastDiff           []map[string]any       `json:"lastDiff,omitempty"`
	Conditions         []metav1.Condition     `json:"conditions,omitempty"`
}
