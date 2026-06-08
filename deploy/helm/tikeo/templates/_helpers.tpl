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

{{- define "tikeo.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "tikeo.name" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "tikeo.httpCertPath" -}}
{{ printf "%s/%s" .Values.server.tls.http.mountPath .Values.tlsDefaults.certFilename }}
{{- end -}}

{{- define "tikeo.httpKeyPath" -}}
{{ printf "%s/%s" .Values.server.tls.http.mountPath .Values.tlsDefaults.keyFilename }}
{{- end -}}

{{- define "tikeo.httpClientCaPath" -}}
{{ printf "%s/%s" .Values.server.tls.http.clientCaMountPath .Values.tlsDefaults.clientCaFilename }}
{{- end -}}

{{- define "tikeo.workerTunnelCertPath" -}}
{{ printf "%s/%s" .Values.server.tls.workerTunnel.mountPath .Values.tlsDefaults.certFilename }}
{{- end -}}

{{- define "tikeo.workerTunnelKeyPath" -}}
{{ printf "%s/%s" .Values.server.tls.workerTunnel.mountPath .Values.tlsDefaults.keyFilename }}
{{- end -}}

{{- define "tikeo.workerTunnelClientCaPath" -}}
{{ printf "%s/%s" .Values.server.tls.workerTunnel.clientCaMountPath .Values.tlsDefaults.clientCaFilename }}
{{- end -}}
