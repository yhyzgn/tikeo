terraform {
  required_version = ">= 1.6.0"
  required_providers {
    tikee = {
      source = "yhyzgn/tikee"
    }
  }
}

provider "tikee" {
  endpoint  = var.tikee_endpoint
  api_token = var.tikee_api_token
}

variable "tikee_endpoint" { type = string }
variable "tikee_api_token" {
  type      = string
  sensitive = true
}

data "tikee_manifest" "current" {
  namespace = "default"
  app       = "billing"
  format    = "yaml"
}

resource "tikee_manifest_diff" "review" {
  manifest_json = data.tikee_manifest.current.manifest_json
}

output "gitops_diff_summary" {
  value = tikee_manifest_diff.review.summary_json
}
