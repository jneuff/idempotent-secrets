{{/*
Format the image field, handling empty registry cases
*/}}
{{- define "create-secret.image" -}}
{{- if .Values.image.repository -}}
{{- printf "%s/%s:%s" .Values.image.repository .Values.image.name .Values.image.tag -}}
{{- else -}}
{{- printf "%s:%s" .Values.image.name .Values.image.tag -}}
{{- end -}}
{{- end -}} 