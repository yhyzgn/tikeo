package main

import (
	"context"
	"log"

	"github.com/hashicorp/terraform-plugin-framework/providerserver"
	"github.com/yhyzgn/tikee/deploy/terraform/provider/internal/provider"
)

func main() {
	err := providerserver.Serve(context.Background(), provider.New, providerserver.ServeOpts{
		Address: "registry.terraform.io/yhyzgn/tikee",
	})
	if err != nil {
		log.Fatal(err)
	}
}
