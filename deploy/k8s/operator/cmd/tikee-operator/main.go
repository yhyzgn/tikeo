package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/yhyzgn/tikee/deploy/k8s/operator/internal/controller"
)

func main() {
	kubeconfig := flag.String("kubeconfig", "", "optional kubeconfig path for out-of-cluster tikee-operator runs")
	endpoint := flag.String("tikee-endpoint", os.Getenv("TIKEE_ENDPOINT"), "tikee management API endpoint")
	token := flag.String("tikee-api-token", os.Getenv("TIKEE_API_TOKEN"), "tikee API token or SDK API-Key")
	flag.Parse()
	if *endpoint == "" || *token == "" {
		fmt.Fprintln(os.Stderr, "tikee-operator requires --tikee-endpoint and --tikee-api-token or TIKEE_ENDPOINT/TIKEE_API_TOKEN")
		os.Exit(2)
	}
	fmt.Printf("tikee-operator starting kubeconfig=%q groupVersion=%s kind=%s endpoint=%s\n", *kubeconfig, controller.GroupVersionString, controller.Kind, *endpoint)
	// The reconciler package owns the testable reconciliation contract. Wiring a controller-runtime Manager
	// can be enabled by deployment packaging without changing CRD/API semantics.
}
