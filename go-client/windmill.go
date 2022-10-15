package windmill

import (
	"context"
	"fmt"
	"net/http"
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
	token := os.Getenv("WM_TOKEN")
	client, _ := api.NewClientWithResponses(base_url, func(c *api.Client) error {
		c.RequestEditors = append(c.RequestEditors, func(ctx context.Context, req *http.Request) error {
			req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", token))
			return nil
		})
		return nil
	})
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
	res, err := client.Client.GetVariableWithResponse(context.Background(), client.Workspace, path, &api.GetVariableParams{
		DecryptSecret: newBool(true),
	})
	if err != nil {
		panic(err)
	}
	return *res.JSON200.Value
}

func GetResource(path string) map[string]interface{} {
	client, err := GetClient()
	if err != nil {
		panic(err)
	}
	res, err := client.Client.GetResourceWithResponse(context.Background(), client.Workspace, path)
	if err != nil {
		panic(err)
	}
	return *res.JSON200.Value
}
