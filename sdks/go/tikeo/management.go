package tikeo

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const apiKeyHeader = "x-tikeo-api-key"

type ManagementClient struct {
	http      *http.Client
	endpoint  string
	apiKey    string
	namespace string
	app       string
}

func NewManagementClient(endpoint, apiKey, namespace, app string) *ManagementClient {
	return &ManagementClient{
		http:      &http.Client{Timeout: 30 * time.Second},
		endpoint:  strings.TrimRight(strings.TrimSpace(endpoint), "/"),
		apiKey:    apiKey,
		namespace: defaultString(namespace, "default"),
		app:       defaultString(app, "default"),
	}
}

type JobDefinition struct {
	ID            string          `json:"id"`
	Namespace     string          `json:"namespace"`
	App           string          `json:"app"`
	Name          string          `json:"name"`
	ScheduleType  string          `json:"scheduleType"`
	ScheduleExpr  *string         `json:"scheduleExpr"`
	ProcessorName *string         `json:"processorName"`
	ProcessorType *string         `json:"processorType"`
	ScriptID      *string         `json:"scriptId"`
	Enabled       bool            `json:"enabled"`
	RetryPolicy   *JobRetryPolicy `json:"retryPolicy,omitempty"`
}

type JobInstance struct {
	ID            string `json:"id"`
	JobID         string `json:"jobId"`
	Status        string `json:"status"`
	TriggerType   string `json:"triggerType"`
	ExecutionMode string `json:"executionMode"`
	CreatedAt     string `json:"createdAt"`
	UpdatedAt     string `json:"updatedAt"`
}

type JobRetryPolicy struct {
	Enabled             bool `json:"enabled"`
	MaxAttempts         int  `json:"maxAttempts"`
	InitialDelaySeconds int  `json:"initialDelaySeconds"`
	BackoffMultiplier   int  `json:"backoffMultiplier"`
	MaxDelaySeconds     int  `json:"maxDelaySeconds"`
}

type BroadcastSelectorRequest struct {
	Tags    []string          `json:"tags,omitempty"`
	Region  string            `json:"region,omitempty"`
	Cluster string            `json:"cluster,omitempty"`
	Labels  map[string]string `json:"labels,omitempty"`
}

func DefaultJobRetryPolicy() *JobRetryPolicy {
	return &JobRetryPolicy{Enabled: true, MaxAttempts: 3, InitialDelaySeconds: 5, BackoffMultiplier: 2, MaxDelaySeconds: 60}
}

type CreateJobRequest struct {
	Name          string
	ScheduleType  string
	ScheduleExpr  *string
	ProcessorName *string
	ProcessorType *string
	WorkerPool    *string
	ScriptID      *string
	Enabled       *bool
	RetryPolicy   *JobRetryPolicy
}

type TriggerJobRequest struct {
	TriggerType       string                    `json:"triggerType,omitempty"`
	ExecutionMode     string                    `json:"executionMode,omitempty"`
	BroadcastSelector *BroadcastSelectorRequest `json:"broadcastSelector,omitempty"`
}

func APITrigger() TriggerJobRequest {
	return TriggerJobRequest{TriggerType: "api", ExecutionMode: "single"}
}

func BroadcastAPITrigger(selector *BroadcastSelectorRequest) TriggerJobRequest {
	return TriggerJobRequest{TriggerType: "api", ExecutionMode: "broadcast", BroadcastSelector: selector}
}

func APIJob(name, processorName string) CreateJobRequest {
	enabled := true
	return CreateJobRequest{
		Name:          name,
		ScheduleType:  "api",
		ProcessorName: stringPtr(processorName),
		Enabled:       &enabled,
		RetryPolicy:   DefaultJobRetryPolicy(),
	}
}

func PluginAPIJob(name, processorType, processorName string) CreateJobRequest {
	enabled := true
	return CreateJobRequest{
		Name:          name,
		ScheduleType:  "api",
		ProcessorType: stringPtr(processorType),
		ProcessorName: stringPtr(processorName),
		Enabled:       &enabled,
		RetryPolicy:   DefaultJobRetryPolicy(),
	}
}

func ScriptAPIJob(name, scriptID string) CreateJobRequest {
	enabled := true
	return CreateJobRequest{
		Name:         name,
		ScheduleType: "api",
		ScriptID:     stringPtr(scriptID),
		Enabled:      &enabled,
		RetryPolicy:  DefaultJobRetryPolicy(),
	}
}

func (r CreateJobRequest) WithWorkerPool(workerPool string) CreateJobRequest {
	if strings.TrimSpace(workerPool) != "" {
		r.WorkerPool = stringPtr(workerPool)
	}
	return r
}

func (c *ManagementClient) ListJobs(ctx context.Context) ([]JobDefinition, error) {
	var page struct {
		Items []JobDefinition `json:"items"`
	}
	if err := c.send(ctx, http.MethodGet, "/jobs", nil, &page); err != nil {
		return nil, err
	}
	out := make([]JobDefinition, 0, len(page.Items))
	for _, job := range page.Items {
		if job.Namespace == c.namespace && job.App == c.app {
			out = append(out, job)
		}
	}
	return out, nil
}

func (c *ManagementClient) CreateJob(ctx context.Context, request CreateJobRequest) (JobDefinition, error) {
	payload := struct {
		Namespace     string          `json:"namespace"`
		App           string          `json:"app"`
		Name          string          `json:"name"`
		ScheduleType  string          `json:"scheduleType,omitempty"`
		ScheduleExpr  *string         `json:"scheduleExpr,omitempty"`
		ProcessorName *string         `json:"processorName,omitempty"`
		ProcessorType *string         `json:"processorType,omitempty"`
		WorkerPool    *string         `json:"workerPool,omitempty"`
		ScriptID      *string         `json:"scriptId,omitempty"`
		Enabled       *bool           `json:"enabled,omitempty"`
		RetryPolicy   *JobRetryPolicy `json:"retryPolicy,omitempty"`
	}{
		Namespace:     c.namespace,
		App:           c.app,
		Name:          request.Name,
		ScheduleType:  request.ScheduleType,
		ScheduleExpr:  request.ScheduleExpr,
		ProcessorName: request.ProcessorName,
		ProcessorType: request.ProcessorType,
		WorkerPool:    request.WorkerPool,
		ScriptID:      request.ScriptID,
		Enabled:       request.Enabled,
		RetryPolicy:   request.RetryPolicy,
	}
	var job JobDefinition
	if err := c.send(ctx, http.MethodPost, "/jobs", payload, &job); err != nil {
		return JobDefinition{}, err
	}
	return job, nil
}

func (c *ManagementClient) TriggerJob(ctx context.Context, jobID string, request TriggerJobRequest) (JobInstance, error) {
	var instance JobInstance
	if err := c.send(ctx, http.MethodPost, "/jobs/"+url.PathEscape(jobID)+":trigger", request, &instance); err != nil {
		return JobInstance{}, err
	}
	return instance, nil
}

func (c *ManagementClient) send(ctx context.Context, method, path string, body any, out any) error {
	var payload *bytes.Reader
	if body == nil {
		payload = bytes.NewReader(nil)
	} else {
		data, err := json.Marshal(body)
		if err != nil {
			return err
		}
		payload = bytes.NewReader(data)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.endpoint+"/api/v1"+path, payload)
	if err != nil {
		return err
	}
	req.Header.Set("accept", "application/json")
	req.Header.Set(apiKeyHeader, c.apiKey)
	if body != nil {
		req.Header.Set("content-type", "application/json")
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	var envelope struct {
		Code    int             `json:"code"`
		Message string          `json:"message"`
		Data    json.RawMessage `json:"data"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&envelope); err != nil {
		return err
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 || envelope.Code != 0 {
		return fmt.Errorf("tikeo management request failed: status=%s message=%s", resp.Status, envelope.Message)
	}
	if out != nil {
		if len(envelope.Data) == 0 || string(envelope.Data) == "null" {
			return fmt.Errorf("tikeo management response data was null")
		}
		if err := json.Unmarshal(envelope.Data, out); err != nil {
			return err
		}
	}
	return nil
}

func defaultString(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return strings.TrimSpace(value)
}

func stringPtr(value string) *string {
	return &value
}
