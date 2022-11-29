import type { Script } from "./gen"

export const PYTHON_INIT_CODE = `import os
import wmill

# You can import any package from PyPI, even if the assistant complains

"""
Use Cmd/Ctrl + S to autoformat the code. Reset content in the bar to start from a clean template.
The client is used to interact with windmill itself through its standard API.
One can explore the methods available through autocompletion of \`wmill.XXX\`.
"""

def main(no_default: str,
         name = "Nicolas Bourbaki",
         age = 42,
         obj: dict = {"even": "dicts"},
         l: list = ["or", "lists!"],
         file_: bytes = bytes(0)):
    """A main function is required for the script to be able to accept arguments.
    Types are recommended."""
    print(f"Hello World and a warm welcome especially to {name}")
    print("and its acolytes..", age, obj, l, len(file_))
    # retrieve variables, including secrets by querying the windmill platform.
    # secret fetching is audited by windmill.

    try:
      secret = wmill.get_variable("g/all/pretty_secret")
    except:
      secret = "No secret yet at g/all/pretty_secret!"

    print(f"The variable at \`g/all/pretty_secret\`: {secret}")
    
    # Get last state of this script execution by the same trigger/user
    last_state = wmill.get_state()
    new_state = {"foo": 42} if last_state is None else last_state
    new_state["foo"] += 1
    wmill.set_state(new_state)

    # fetch reserved variables as environment variables
    user = os.environ.get("WM_USERNAME")
    # the return value is then parsed and can be retrieved by other scripts conveniently
    return {"splitted": name.split(), "user": user, "state": new_state}
`
export const DENO_INIT_CODE = `// reload the smart assistant on the top right if it dies to get autocompletion and syntax highlighting
// (Ctrl+space to cache dependencies on imports hover, Ctrl+S to autoformat and parse parameters).

// you can use npm imports directly
// import { toWords } from "npm:number-to-words@1"
// import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main(
  a: number,
  b: "my" | "enum",
  d = "inferred type string from default arg",
  c = { nested: "object" },
  //e: wmill.Base64
) {
  // let x = await wmill.getVariable('u/user/foo')
  return { foo: a };
}
`

export const GO_INIT_CODE = `package inner

import (
	"fmt"
	"rsc.io/quote"
  // wmill "github.com/windmill-labs/windmill-go-client"
)

// the main must return (interface{}, error)

func main(x string, nested struct{ Foo string \`json:"foo"\` }) (interface{}, error) {
	fmt.Println("Hello, World")
	fmt.Println(nested.Foo)
	fmt.Println(quote.Opt())
  // v, _ := wmill.GetVariable("g/all/pretty_secret")
  return x, nil
}
`

export const GO_FAILURE_MODULE_CODE = `package inner

import (
	"fmt"
  "os"
)

// connect the error parameter to 'previous_result.error'

func main(error string) (interface{}, error) {
	fmt.Println(error)
	fmt.Println("job", os.Getenv("WM_JOB_ID"))
  return x, nil
}
`

export const DENO_INIT_CODE_CLEAR = `// import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main(x: string) {
  return x
}
`

export const DENO_FAILURE_MODULE_CODE = `
// connect the error parameter to 'previous_result.error'

export async function main(error: string) {
  const job = Deno.env.get("WM_JOB_ID")
  console.log("error", error)
  console.log("job", job)
  return { error, job }
}
`

export const PYTHON_INIT_CODE_CLEAR = `#import wmill

def main(x: str):
  return x
`

export const PYTHON_FAILURE_MODULE_CODE = `import os

# connect the error parameter to 'previous_result.error'

def main(error: str):
  job = os.environ.get("WM_JOB_ID")
  print("error", error)
  print("job", job)
  return error, job
`

export const POSTGRES_INIT_CODE = `import {
  pgSql,
  type Resource,
} from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts";

//PG parameterized statement. No SQL injection is possible.
export async function main(
  db: Resource<"postgresql"> = "$res:g/all/demodb",
  key: number,
  value: string,
) {
  const query = await pgSql(
    db,
  )\`INSERT INTO demo VALUES (\${key}, \${value}) RETURNING *\`;
  return query.rows;
}`

export const BASH_INIT_CODE = `
# arguments of the form X="$I" are parsed as parameters X of type string
msg="$1"

# the last line of the stdout is the return value
echo "Hello $msg"
`


export const DENO_INIT_CODE_TRIGGER = `import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main() {

  // A common trigger script would follow this pattern:
  // 1. Get the last saved state
  // const state = await wmill.getState()
  // 2. Get the actual state from the external service
  // const newState = await (await fetch('https://hacker-news.firebaseio.com/v0/topstories.json')).json()
  // 3. Compare the two states and update the internal state
  // await wmill.setState(newState)
  // 4. Return the new rows
  // return range from (state to newState)

  return [1,2,3]

  // In subsequent scripts, you may refer to each row/value returned by the trigger script using
  // 'flow_input.iter.value'
}
`

export const PYTHON_INIT_CODE_TRIGGER = `import wmill

def main():
    # A common trigger script would follow this pattern:
    # 1. Get the last saved state
    # const state = wnmill.get_state()
    # 2. Get the actual state from the external service
    # const newState = ...
    # 3. Compare the two states and update the internal state
    # wmill.setState(newState)
    # 4. Return the new rows
    # return range from (state to newState)
    return [1, 2, 3]
`


export const GO_INIT_CODE_TRIGGER = `package inner

import (
	wmill "github.com/windmill-labs/windmill-go-client"
)

func main() (interface{}, error) {

	// A common trigger script would follow this pattern:
	// 1. Get the last saved state
	state, _ := wmill.GetState()
	// 2. Get the actual state from the external service
	// newState := ...
	// 3. Compare the two states and update the internal state
	wmill.SetState(4)
	// 4. Return the new rows

	return state, nil

	// In subsequent scripts, you may refer to each row/value returned by the trigger script using
	// 'flow_input.iter.value'
}
`

export const DENO_INIT_CODE_APPROVAL = `import * as wmill from "https://deno.land/x/windmill@v1.41.0/mod.ts"

export async function main(approver?: string) {
  return wmill.getResumeEndpoints(approver)
}`

const ALL_INITIAL_CODE = [PYTHON_INIT_CODE, PYTHON_INIT_CODE_TRIGGER, DENO_INIT_CODE, POSTGRES_INIT_CODE, DENO_INIT_CODE_TRIGGER, DENO_INIT_CODE_CLEAR, PYTHON_INIT_CODE_CLEAR, DENO_INIT_CODE_APPROVAL, DENO_FAILURE_MODULE_CODE]

export function isInitialCode(content: string): boolean {
  for (const code of ALL_INITIAL_CODE) {
    if (content === code) {
      return true
    }
  }
  return false
}

export function initialCode(language: 'deno' | 'python3' | 'go' | 'bash', kind: Script.kind, subkind: 'pgsql' | 'flow' | 'script' | undefined): string {
  if (language === 'deno') {
    if (kind === 'trigger') {
      return DENO_INIT_CODE_TRIGGER
    } else if (kind === 'script') {
      if (subkind === 'flow') {
        return DENO_INIT_CODE_CLEAR
      }
      else if (subkind === 'pgsql') {
        return POSTGRES_INIT_CODE
      } else {
        return DENO_INIT_CODE
      }
    } else if (kind === 'failure') {
      return DENO_FAILURE_MODULE_CODE
    } else if (kind === 'approval') {
      return DENO_INIT_CODE_APPROVAL
    }
    else {
      return DENO_INIT_CODE
    }
  } else if (language === 'python3') {
    if (subkind === 'flow') {
      return PYTHON_INIT_CODE_CLEAR
    } else if (kind === 'failure') {
      return PYTHON_FAILURE_MODULE_CODE
    } else if (kind === 'trigger') {
      return PYTHON_INIT_CODE_TRIGGER
    } else {
      return PYTHON_INIT_CODE
    }
  } else if (language == 'bash') {
    return BASH_INIT_CODE
  } else {
    if (kind === 'failure') {
      return GO_FAILURE_MODULE_CODE
    } else if (kind === 'trigger') {
      return GO_INIT_CODE_TRIGGER
    } else {
      return GO_INIT_CODE
    }
  }
}
