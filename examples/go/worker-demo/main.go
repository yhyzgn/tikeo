package main

import (
	"context"
	"fmt"
	"log"

	tikee "github.com/yhyzgn/tikee/sdks/go/tikee"
)

func main() {
	config := tikee.LocalConfig("http://127.0.0.1:9998", "go-demo-worker")
	config.Namespace = "default"
	config.App = "demo"
	config.Capabilities = []string{"echo"}

	client, err := tikee.NewClient(config)
	if err != nil {
		log.Fatal(err)
	}
	processor := tikee.TaskProcessorFunc(func(context.Context, tikee.TaskContext) (tikee.TaskOutcome, error) {
		return tikee.Succeeded(), nil
	})
	if err := client.StartDryRun(context.Background(), processor); err != nil {
		log.Fatal(err)
	}
	heartbeat, err := client.NextHeartbeat("dry-run-worker", "dry-run-fence", 1)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("registration=%+v heartbeat_sequence=%d\n", client.Registration(), heartbeat.Sequence)
}
