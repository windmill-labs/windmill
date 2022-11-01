package inner

import (
	"fmt"
	"rsc.io/quote"
  // wmill "github.com/windmill-labs/windmill-go-client"
)

func main(x string) (interface{}, error) {
	fmt.Println("Hello, World")
	fmt.Println(quote.Opt())
  // v, _ := wmill.GetVariable("g/all/pretty_secret")
  return x, nil
}
