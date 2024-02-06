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
	res, err := client.Client.GetVariableValueWithResponse(context.Background(), client.Workspace, path)
	if err != nil {
		return "", err
	}
	if res.StatusCode()/100 != 2 {
		return "", errors.New(string(res.Body))
	}
	return *res.JSON200, nil
}

func GetResource(path string) (interface{}, error) {
	client, err := GetClient()
	if err != nil {
		return nil, err
	}
	params := api.GetResourceValueInterpolatedParams{}
	res, err := client.Client.GetResourceValueInterpolatedWithResponse(context.Background(), client.Workspace, path, &params)
	if err != nil {
		return nil, err
	}
	if res.StatusCode()/100 != 2 {
		return nil, errors.New(string(res.Body))
	}
	return *res.JSON200, nil
}

func SetResource(path string, value interface{}, resourceTypeOpt ...string) error {
	client, err := GetClient()
	if err != nil {
		return err
	}
	params := api.GetResourceValueInterpolatedParams{}
	getRes, getErr := client.Client.GetResourceValueInterpolatedWithResponse(context.Background(), client.Workspace, path, &params)
	if getErr != nil {
		return getErr
	}
	if getRes.StatusCode() == 404 {
		resourceType := "any"
		if len(resourceTypeOpt) > 0 {
			resourceType = resourceTypeOpt[0]
		}
		res, err := client.Client.CreateResourceWithResponse(context.Background(), client.Workspace, &api.CreateResourceParams{
			UpdateIfExists: newBool(true),
		}, api.CreateResource{Value: &value, Path: path, ResourceType: resourceType})
		if err != nil {
			return err
		}
		if res.StatusCode()/100 != 2 {
			return errors.New(string(res.Body))
		}
	} else {
		res, err := client.Client.UpdateResourceValueWithResponse(context.Background(), client.Workspace, path, api.UpdateResourceValueJSONRequestBody{
			Value: &value,
		})
		if err != nil {
			return err
		}
		if res.StatusCode()/100 != 2 {
			return errors.New(string(res.Body))
		}
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
		f = true
	}
	if res.StatusCode()/100 != 2 {
		f = true
	}
	if f {
		res, err := client.Client.CreateVariableWithResponse(context.Background(), client.Workspace, &api.CreateVariableParams{},
			api.CreateVariableJSONRequestBody{
				Path:  path,
				Value: value,
			})

		if err != nil {
			return err
		}
		if res.StatusCode()/100 != 2 {
			return errors.New(string(res.Body))
		}
	}
	return nil
}

func GetStatePath() string {
	value := os.Getenv("WM_STATE_PATH_NEW")
	if len(value) == 0 {
		return os.Getenv("WM_STATE_PATH")
	}
	return value
}

func GetState() (interface{}, error) {
	return GetResource(GetStatePath())
}

func SetState(state interface{}) error {
	err := SetResource(GetStatePath(), state)
	if err != nil {
		return err
	}
	return nil
}
