package windmill

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/stretchr/testify/require"
)

func SetUp() error {
	err := os.Setenv("BASE_INTERNAL_URL", "http://localhost:8000")
	if err != nil {
		return err
	}
	err = os.Setenv("WM_WORKSPACE", "storage")
	if err != nil {
		return err
	}
	err = os.Setenv("WM_TOKEN", "<WM_TOKEN>")
	if err != nil {
		return err
	}
	return nil
}

func TestGetResource(t *testing.T) {
	t.Skip("skipping") // uncomment to test
	if err := SetUp(); err != nil {
		t.Error(err)
	}
	res, err := GetResource("u/admin/test_res")
	if err != nil {
		t.Error(err)
	}
	serialized, err := json.Marshal(res)
	if err != nil {
		t.Error(err)
	}
	require.Equal(t, "{\"test\":\"test\"}", string(serialized))
}

func TestSetResource(t *testing.T) {
	t.Skip("skipping") // uncomment to test
	if err := SetUp(); err != nil {
		t.Error(err)
	}
	path := "u/admin/test_res"

	type ResourceValue struct {
		Test string `json:"test"`
	}
	value := ResourceValue{
		Test: "test3",
	}
	if err := SetResource(path, value); err != nil {
		t.Error(err)
	}
}
