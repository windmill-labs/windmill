package windmill

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"

	api "github.com/windmill-labs/windmill-go-client/api"
)

type ClientWithWorkspace struct {
	Client    *api.ClientWithResponses
	Workspace string
}

func GetClient() (ClientWithWorkspace, error) {
	base_url := os.Getenv("BASE_INTERNAL_URL") + "/api"
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

func GetVariable(path string) (string, error) {
	client, err := GetClient()
	if err != nil {
		return "", err
	}
	res, err := client.Client.GetVariableWithResponse(context.Background(), client.Workspace, path, &api.GetVariableParams{
		DecryptSecret: newBool(true),
	})
	if res.StatusCode()/100 != 2 {
		return "", errors.New(string(res.Body))
	}
	if err != nil {
		return "", err
	}
	return *res.JSON200.Value, nil
}

func GetResource(path string) (interface{}, error) {
	client, err := GetClient()
	if err != nil {
		return nil, err
	}
	res, err := client.Client.GetResourceWithResponse(context.Background(), client.Workspace, path)
	if res.StatusCode()/100 != 2 {
		return nil, errors.New(string(res.Body))
	}
	if err != nil {
		return nil, err
	}
	return *res.JSON200.Value, nil
}

func SetResource(path string, value interface{}) error {
	client, err := GetClient()
	if err != nil {
		return err
	}
	res, err := client.Client.UpdateResourceValueWithResponse(
		context.Background(),
		client.Workspace, path,
		api.UpdateResourceValueJSONRequestBody{Value: &value})
	if err != nil {
		return err
	}
	if res.StatusCode()/100 != 2 {
		return errors.New(string(res.Body))
	}
	return nil
}

func SetVariable(path string, value string) error {
	client, err := GetClient()
	if err != nil {
		return err
	}
	f := false
	res, err := client.Client.UpdateVariableWithResponse(context.Background(), client.Workspace, path, &api.UpdateVariableParams{AlreadyEncrypted: &f}, api.EditVariable{Value: &value})
	if err != nil {
		return err
	}
	if res.StatusCode()/100 != 2 {
		return errors.New(string(res.Body))
	}
	return nil
}

func GetStatePath() string {
	return os.Getenv("WM_STATE_PATH")
}

func GetState() (interface{}, error) {
	return GetResource(GetStatePath())
}

func SetState(state interface{}) error {
	err := SetResource(GetStatePath(), state)
	if err != nil {
		client, err := GetClient()
		if err != nil {
			return err
		}
		res, err := client.Client.CreateResourceWithResponse(context.Background(), client.Workspace, api.CreateResource{Value: &state})
		if err != nil {
			return err
		}
		if res.StatusCode()/100 != 2 {
			return errors.New(string(res.Body))
		}
		return nil
	}
	return nil
}
