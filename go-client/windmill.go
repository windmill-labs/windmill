package windmill

import (
	"context"
	"fmt"
	"os"

	api "github.com/windmill-labs/windmill-go-client/api"
)

func hello_world() {
	fmt.Println("Windmill")
}

type ClientWithWorkspace struct {
	Client    *api.ClientWithResponses
	Workspace string
}

func GetClient() (ClientWithWorkspace, error) {
	base_url := os.Getenv("BASE_INTERNAL_URL")
	workspace := os.Getenv("WM_WORKSPACE")
	client, _ := api.NewClientWithResponses(base_url)
	return ClientWithWorkspace{
		Client:    client,
		Workspace: workspace,
	}, nil
}
func newBool(b bool) *bool {
	return &b
}

func GetVariable(path string) string {
	client, err := GetClient()
	if err != nil {
		panic(err)
	}
	res, _ := client.Client.GetVariableWithResponse(context.Background(), client.Workspace, path, &api.GetVariableParams{
		DecryptSecret: newBool(true),
	})
	return *res.JSON200.Value
}

func GetResource(path string) map[string]interface{} {
	client, err := GetClient()
	if err != nil {
		panic(err)
	}
	res, _ := client.Client.GetResourceWithResponse(context.Background(), client.Workspace, path)
	return *res.JSON200.Value
}
