package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/yhyzgn/tikeo/deploy/k8s/operator/internal/controller"
)

func main() {
	kubeconfig := flag.String("kubeconfig", "", "optional kubeconfig path for out-of-cluster tikeo-operator runs")
	endpoint := flag.String("tikeo-endpoint", os.Getenv("TIKEO_ENDPOINT"), "tikeo management API endpoint")
	token := flag.String("tikeo-api-token", os.Getenv("TIKEO_API_TOKEN"), "tikeo API token or SDK API-Key")
	flag.Parse()
	if *endpoint == "" || *token == "" {
		fmt.Fprintln(os.Stderr, "tikeo-operator requires --tikeo-endpoint and --tikeo-api-token or TIKEO_ENDPOINT/TIKEO_API_TOKEN")
		os.Exit(2)
	}
	fmt.Printf("tikeo-operator starting kubeconfig=%q groupVersion=%s kind=%s endpoint=%s\n", *kubeconfig, controller.GroupVersionString, controller.Kind, *endpoint)
	// The reconciler package owns the testable reconciliation contract. Wiring a controller-runtime Manager
	// can be enabled by deployment packaging without changing CRD/API semantics.
}
