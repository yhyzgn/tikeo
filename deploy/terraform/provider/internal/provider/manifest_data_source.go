package provider

import (
	"context"
	"encoding/json"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/datasource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type manifestDataSource struct{ client *configuredClient }

type manifestModel struct {
	Namespace    types.String `tfsdk:"namespace"`
	App          types.String `tfsdk:"app"`
	Format       types.String `tfsdk:"format"`
	Checksum     types.String `tfsdk:"checksum"`
	ManifestJSON types.String `tfsdk:"manifest_json"`
	ManifestYAML types.String `tfsdk:"manifest_yaml"`
}

const manifestDataSourceName = "tikeo_manifest"

func NewManifestDataSource() datasource.DataSource { return &manifestDataSource{} }

func (d *manifestDataSource) Metadata(_ context.Context, request datasource.MetadataRequest, response *datasource.MetadataResponse) {
	response.TypeName = request.ProviderTypeName + "_manifest"
}

func (d *manifestDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, response *datasource.SchemaResponse) {
	response.Schema = schema.Schema{Description: "Exports the current tikeo GitOps manifest via /api/v1/gitops/manifest.", Attributes: map[string]schema.Attribute{
		"namespace": schema.StringAttribute{Optional: true},
		"app": schema.StringAttribute{Optional: true},
		"format": schema.StringAttribute{Optional: true},
		"checksum": schema.StringAttribute{Computed: true},
		"manifest_json": schema.StringAttribute{Computed: true},
		"manifest_yaml": schema.StringAttribute{Computed: true},
	}}
}

func (d *manifestDataSource) Configure(_ context.Context, request datasource.ConfigureRequest, _ *datasource.ConfigureResponse) {
	if request.ProviderData != nil { d.client = request.ProviderData.(*configuredClient) }
}

func (d *manifestDataSource) Read(ctx context.Context, request datasource.ReadRequest, response *datasource.ReadResponse) {
	var state manifestModel
	response.Diagnostics.Append(request.Config.Get(ctx, &state)...)
	if response.Diagnostics.HasError() { return }
	format := state.Format.ValueString()
	if format == "" { format = "json" }
	payload, err := d.client.client.ExportManifest(ctx, state.Namespace.ValueString(), state.App.ValueString(), format)
	if err != nil { response.Diagnostics.AddError("Unable to export tikeo manifest", err.Error()); return }
	var envelope struct {
		Manifest     json.RawMessage `json:"manifest"`
		ManifestYAML string          `json:"manifestYaml"`
		Checksum     string          `json:"checksum"`
	}
	if err := json.Unmarshal(payload, &envelope); err != nil { response.Diagnostics.AddError("Invalid tikeo manifest response", err.Error()); return }
	state.Checksum = types.StringValue(envelope.Checksum)
	state.ManifestJSON = types.StringValue(string(envelope.Manifest))
	state.ManifestYAML = types.StringValue(envelope.ManifestYAML)
	response.Diagnostics.Append(response.State.Set(ctx, &state)...)
}
