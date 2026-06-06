terraform {
  required_version = ">= 1.6.0"
  required_providers {
    tikeo = {
      source = "yhyzgn/tikeo"
    }
  }
}

provider "tikeo" {
  endpoint  = var.tikeo_endpoint
  api_token = var.tikeo_api_token
}

variable "tikeo_endpoint" { type = string }
variable "tikeo_api_token" {
  type      = string
  sensitive = true
}

data "tikeo_manifest" "current" {
  namespace = "default"
  app       = "billing"
  format    = "yaml"
}

resource "tikeo_manifest_diff" "review" {
  manifest_json = data.tikeo_manifest.current.manifest_json
}

output "gitops_diff_summary" {
  value = tikeo_manifest_diff.review.summary_json
}
