package provider

import (
	"context"
	"os"
	"time"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/provider"
	"github.com/hashicorp/terraform-plugin-framework/provider/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/types"
	"github.com/yhyzgn/tikeo/deploy/terraform/provider/internal/tikeo"
)

func New() provider.Provider { return &TikeoProvider{} }

type TikeoProvider struct{}

type model struct {
	Endpoint types.String `tfsdk:"endpoint"`
	APIToken types.String `tfsdk:"api_token"`
	Timeout  types.Int64  `tfsdk:"timeout_seconds"`
}

type configuredClient struct{ client *tikeo.Client }

func (p *TikeoProvider) Metadata(_ context.Context, _ provider.MetadataRequest, response *provider.MetadataResponse) {
	response.TypeName = "tikeo"
}

func (p *TikeoProvider) Schema(_ context.Context, _ provider.SchemaRequest, response *provider.SchemaResponse) {
	response.Schema = schema.Schema{Attributes: map[string]schema.Attribute{
		"endpoint": schema.StringAttribute{Optional: true, Description: "Base URL for the tikeo management API. Can also be set with TIKEO_ENDPOINT."},
		"api_token": schema.StringAttribute{Optional: true, Sensitive: true, Description: "API token or SDK API-Key for tikeo management APIs. Can also be set with TIKEO_API_TOKEN."},
		"timeout_seconds": schema.Int64Attribute{Optional: true, Description: "HTTP request timeout in seconds. Defaults to 30."},
	}}
}

func (p *TikeoProvider) Configure(ctx context.Context, request provider.ConfigureRequest, response *provider.ConfigureResponse) {
	var config model
	response.Diagnostics.Append(request.Config.Get(ctx, &config)...)
	if response.Diagnostics.HasError() { return }
	endpoint := os.Getenv("TIKEO_ENDPOINT")
	if !config.Endpoint.IsNull() && !config.Endpoint.IsUnknown() { endpoint = config.Endpoint.ValueString() }
	token := os.Getenv("TIKEO_API_TOKEN")
	if !config.APIToken.IsNull() && !config.APIToken.IsUnknown() { token = config.APIToken.ValueString() }
	timeout := int64(30)
	if !config.Timeout.IsNull() && !config.Timeout.IsUnknown() { timeout = config.Timeout.ValueInt64() }
	client, err := tikeo.NewClient(tikeo.Config{Endpoint: endpoint, APIToken: token, Timeout: time.Duration(timeout) * time.Second})
	if err != nil {
		response.Diagnostics.AddAttributeError(path.Root("endpoint"), "Invalid tikeo provider configuration", err.Error())
		return
	}
	configured := &configuredClient{client: client}
	response.DataSourceData = configured
	response.ResourceData = configured
}

func (p *TikeoProvider) DataSources(context.Context) []func() datasource.DataSource {
	return []func() datasource.DataSource{NewManifestDataSource}
}

func (p *TikeoProvider) Resources(context.Context) []func() resource.Resource {
	return []func() resource.Resource{NewManifestDiffResource}
}
