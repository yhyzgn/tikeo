terraform {
  required_version = ">= 1.6.0"
  required_providers {
    tikeo = {
      source = "yhyzgn/tikeo"
    }
  }
}

variable "tikeo_api_base" { type = string }
variable "tikeo_api_token" {
  type      = string
  sensitive = true
}

provider "tikeo" {
  endpoint  = var.tikeo_api_base
  api_token = var.tikeo_api_token
}

data "tikeo_manifest" "current" {
  format = "yaml"
}

resource "tikeo_manifest_diff" "review" {
  manifest_json = data.tikeo_manifest.current.manifest_json
}

output "gitops_contract" {
  value = {
    checksum = data.tikeo_manifest.current.checksum
    summary  = tikeo_manifest_diff.review.summary_json
  }
}
