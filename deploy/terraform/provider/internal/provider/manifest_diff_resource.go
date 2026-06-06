package provider

import (
	"context"
	"encoding/json"

	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type manifestDiffResource struct{ client *configuredClient }

type diffModel struct {
	ID              types.String `tfsdk:"id"`
	ManifestJSON    types.String `tfsdk:"manifest_json"`
	CurrentChecksum types.String `tfsdk:"current_checksum"`
	DesiredChecksum types.String `tfsdk:"desired_checksum"`
	SummaryJSON      types.String `tfsdk:"summary_json"`
	ChangesJSON      types.String `tfsdk:"changes_json"`
}

const manifestDiffResourceName = "tikeo_manifest_diff"

func NewManifestDiffResource() resource.Resource { return &manifestDiffResource{} }

func (r *manifestDiffResource) Metadata(_ context.Context, request resource.MetadataRequest, response *resource.MetadataResponse) {
	response.TypeName = request.ProviderTypeName + "_manifest_diff"
}

func (r *manifestDiffResource) Schema(_ context.Context, _ resource.SchemaRequest, response *resource.SchemaResponse) {
	response.Schema = schema.Schema{Description: "Stores desired manifest JSON and records /api/v1/gitops/diff output. It is diff/apply-review only; typed CRUD APIs remain the mutation path.", Attributes: map[string]schema.Attribute{
		"id": schema.StringAttribute{Computed: true},
		"manifest_json": schema.StringAttribute{Required: true},
		"current_checksum": schema.StringAttribute{Computed: true},
		"desired_checksum": schema.StringAttribute{Computed: true},
		"summary_json": schema.StringAttribute{Computed: true},
		"changes_json": schema.StringAttribute{Computed: true},
	}}
}

func (r *manifestDiffResource) Configure(_ context.Context, request resource.ConfigureRequest, _ *resource.ConfigureResponse) {
	if request.ProviderData != nil { r.client = request.ProviderData.(*configuredClient) }
}

func (r *manifestDiffResource) Create(ctx context.Context, request resource.CreateRequest, response *resource.CreateResponse) {
	var plan diffModel
	response.Diagnostics.Append(request.Plan.Get(ctx, &plan)...)
	if response.Diagnostics.HasError() { return }
	r.refresh(ctx, &plan, &response.Diagnostics)
	if response.Diagnostics.HasError() { return }
	response.Diagnostics.Append(response.State.Set(ctx, &plan)...)
}

func (r *manifestDiffResource) Read(ctx context.Context, request resource.ReadRequest, response *resource.ReadResponse) {
	var state diffModel
	response.Diagnostics.Append(request.State.Get(ctx, &state)...)
	if response.Diagnostics.HasError() { return }
	r.refresh(ctx, &state, &response.Diagnostics)
	if response.Diagnostics.HasError() { return }
	response.Diagnostics.Append(response.State.Set(ctx, &state)...)
}

func (r *manifestDiffResource) Update(ctx context.Context, request resource.UpdateRequest, response *resource.UpdateResponse) {
	var plan diffModel
	response.Diagnostics.Append(request.Plan.Get(ctx, &plan)...)
	if response.Diagnostics.HasError() { return }
	r.refresh(ctx, &plan, &response.Diagnostics)
	if response.Diagnostics.HasError() { return }
	response.Diagnostics.Append(response.State.Set(ctx, &plan)...)
}

func (r *manifestDiffResource) Delete(ctx context.Context, _ resource.DeleteRequest, response *resource.DeleteResponse) {
	response.State.RemoveResource(ctx)
}

func (r *manifestDiffResource) refresh(ctx context.Context, state *diffModel, diagnostics interface{ AddError(string, string) }) {
	payload, err := r.client.client.DiffManifest(ctx, json.RawMessage(state.ManifestJSON.ValueString()))
	if err != nil { diagnostics.AddError("Unable to diff tikeo manifest", err.Error()); return }
	var diff struct {
		CurrentChecksum string          `json:"currentChecksum"`
		DesiredChecksum string          `json:"desiredChecksum"`
		Summary         json.RawMessage `json:"summary"`
		Changes         json.RawMessage `json:"changes"`
	}
	if err := json.Unmarshal(payload, &diff); err != nil { diagnostics.AddError("Invalid tikeo diff response", err.Error()); return }
	state.ID = types.StringValue(diff.DesiredChecksum)
	state.CurrentChecksum = types.StringValue(diff.CurrentChecksum)
	state.DesiredChecksum = types.StringValue(diff.DesiredChecksum)
	state.SummaryJSON = types.StringValue(string(diff.Summary))
	state.ChangesJSON = types.StringValue(string(diff.Changes))
}
