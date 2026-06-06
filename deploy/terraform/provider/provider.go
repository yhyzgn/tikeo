package main

import (
	"context"
	"log"

	"github.com/hashicorp/terraform-plugin-framework/providerserver"
	"github.com/yhyzgn/tikeo/deploy/terraform/provider/internal/provider"
)

func main() {
	err := providerserver.Serve(context.Background(), provider.New, providerserver.ServeOpts{
		Address: "registry.terraform.io/yhyzgn/tikeo",
	})
	if err != nil {
		log.Fatal(err)
	}
}
