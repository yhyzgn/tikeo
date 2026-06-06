{{- define "tikeo.name" -}}
tikeo
{{- end -}}

{{- define "tikeo.namespace" -}}
{{- default .Release.Namespace .Values.namespaceOverride -}}
{{- end -}}

{{- define "tikeo.labels" -}}
app.kubernetes.io/name: {{ include "tikeo.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version | replace "+" "_" }}
{{- end -}}
