import { Script } from './gen'

import PYTHON_INIT_CODE from '$lib/init_scripts/python_init_code'
import PYTHON_INIT_CODE_CLEAR from '$lib/init_scripts/python_init_code_clear'
import PYTHON_INIT_CODE_TRIGGER from '$lib/init_scripts/python_init_code_trigger'
import PYTHON_FAILURE_MODULE_CODE from '$lib/init_scripts/python_failure_module'
import type { SupportedLanguage } from './common'

export {
	PYTHON_INIT_CODE,
	PYTHON_INIT_CODE_CLEAR,
	PYTHON_INIT_CODE_TRIGGER,
	PYTHON_FAILURE_MODULE_CODE
}

export const NATIVETS_INIT_CODE = `// Fetch-only script, no imports allowed but benefits from a dedicated highly efficient runtime

export async function main(example_input: number = 3) {
  // "3" is the default value of example_input, it can be overriden with code or using the UI
  const res = await fetch(\`https://jsonplaceholder.typicode.com/todos/\${example_input}\`, {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
`

export const NATIVETS_INIT_CODE_CLEAR = `// Fetch-only script, no imports allowed but benefits from a dedicated highly efficient runtime

export async function main() {
  const res = await fetch("https://jsonplaceholder.typicode.com/todos/1", {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
`

export const DENO_INIT_CODE = `// Ctrl/CMD+. to cache dependencies on imports hover.

// Deno uses "npm:" prefix to import from npm (https://deno.land/manual@v1.36.3/node/npm_specifiers)
// import * as wmill from "npm:windmill-client@1"

// fill the type, or use the +Resource type to get a type-safe reference to a resource
// type Postgresql = object

export async function main(
  a: number,
  b: "my" | "enum",
  //c: Postgresql,
  d = "inferred type string from default arg",
  e = { nested: "object" },
  //e: wmill.Base64
) {
  // let x = await wmill.getVariable('u/user/foo')
  return { foo: a };
}
`

export const BUN_INIT_CODE = `// import { toWords } from "number-to-words@1"
import * as wmill from "windmill-client"

// fill the type, or use the +Resource type to get a type-safe reference to a resource
// type Postgresql = object

export async function main(
  a: number,
  b: "my" | "enum",
  //c: Postgresql,
  d = "inferred type string from default arg",
  e = { nested: "object" },
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

// Pin dependencies partially in go.mod with a comment starting with "//require":
//require rsc.io/quote v1.5.1

// the main must return (interface{}, error)

func main(x string, nested struct {
	Foo string \`json:"foo"\`
}) (interface{}, error) {
	fmt.Println("Hello, World")
	fmt.Println(nested.Foo)
	fmt.Println(quote.Opt())
	// v, _ := wmill.GetVariable("f/examples/secret")
	return x, nil
}
`

export const GO_FAILURE_MODULE_CODE = `package inner

import (
	"fmt"
  "os"
)

// connect the error parameter to 'previous_result.error'

func main(message string, name string) (interface{}, error) {
	fmt.Println(message)
	fmt.Println(name)
	fmt.Println("flow id that failed", os.Getenv("WM_FLOW_JOB_ID"))
  return message, nil
}
`

export const DENO_INIT_CODE_CLEAR = `// import * as wmill from "npm:windmill-client@1"

export async function main(x: string) {
  return x
}
`

export const BUN_INIT_CODE_CLEAR = `// import * as wmill from "windmill-client"

export async function main(x: string) {
  return x
}
`

export const DENO_FAILURE_MODULE_CODE = `
export async function main(message: string, name: string) {
  const flow_id = Deno.env.get("WM_FLOW_JOB_ID")
  console.log("message", message)
  console.log("name",name)
  return { message, flow_id }
}
`

export const POSTGRES_INIT_CODE = `-- $1 name1 = default arg
-- $2 name2
-- $3 name3
INSERT INTO demo VALUES (\$1::TEXT, \$2::INT, \$3::TEXT[]) RETURNING *
`

export const MYSQL_INIT_CODE = `-- ? name1 (text) = default arg
-- ? name2 (int)
INSERT INTO demo VALUES (?, ?)
`

export const BIGQUERY_INIT_CODE = `-- @name1 (string) = default arg
-- @name2 (integer)
-- @name3 (string[])
INSERT INTO \`demodb.demo\` VALUES (@name1, @name2, @name3)
`

export const SNOWFLAKE_INIT_CODE = `-- ? name1 (varchar) = default arg
-- ? name2 (int)
INSERT INTO demo VALUES (?, ?)
`

export const MSSQL_INIT_CODE = `-- @p1 name1 (varchar) = default arg
-- @p2 name2 (int)
INSERT INTO demo VALUES (@p1, @p2)
`

export const GRAPHQL_INIT_CODE = `query($name4: String, $name2: Int, $name3: [String]) {
	demo(name1: $name1, name2: $name2, name3: $name3) {
		name1,
		name2,
		name3
	}
}
`

export const FETCH_INIT_CODE = `export async function main(
	url: string | undefined,
	method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'OPTIONS' = 'GET',
	body: Object = {},
	headers: Record<string, string> = {}
): Promise<Response | null> {
	if (!url) {
		console.error('Error: URL is undefined')
		return null
	}

	const requestOptions: RequestInit = {
		method: method || 'GET',
		headers: headers || {}
	}

	if (requestOptions.method !== 'GET' && requestOptions.method !== 'HEAD' && body !== undefined) {
		requestOptions.body = JSON.stringify(body)
		requestOptions.headers = {
			'Content-Type': 'application/json',
			...requestOptions.headers
		}
	}

	return await fetch(url, requestOptions)
		.then((res) => res.json())
		.catch(() => {
			throw new Error('An error occured')
		})
}`

export const BASH_INIT_CODE = `# shellcheck shell=bash
# arguments of the form X="$I" are parsed as parameters X of type string
msg="$1"
dflt="\${2:-default value}"

# the last line of the stdout is the return value
# unless you write json to './result.json' or a string to './result.out'
echo "Hello $msg"
`

export const DENO_INIT_CODE_TRIGGER = `import * as wmill from "npm:windmill-client@1"

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

export const DENO_INIT_CODE_APPROVAL = `import * as wmill from "npm:windmill-client@^1.158.2"

export async function main(approver?: string) {
  return wmill.getResumeUrls(approver)
}`

export const BUN_INIT_CODE_APPROVAL = `import * as wmill from "windmill-client@^1.158.2"

export async function main(approver?: string) {
  return wmill.getResumeUrls(approver)
}`

export const DOCKER_INIT_CODE = `# shellcheck shell=bash
# Bash script that calls docker as a client to the host daemon
# See documentation: https://www.windmill.dev/docs/advanced/docker
msg="\${1:-world}"

IMAGE="alpine:latest"
COMMAND="/bin/echo Hello $msg"

# ensure that the image is up-to-date
docker pull $IMAGE
docker run --rm $IMAGE $COMMAND
`

export const POWERSHELL_INIT_CODE = `param($Msg, $Dflt = "default value", [int]$Nb = 3)

# the last line of the stdout is the return value
Write-Output "Hello $Msg"`

const ALL_INITIAL_CODE = [
	PYTHON_INIT_CODE,
	PYTHON_INIT_CODE_TRIGGER,
	DENO_INIT_CODE,
	POSTGRES_INIT_CODE,
	MYSQL_INIT_CODE,
	BIGQUERY_INIT_CODE,
	SNOWFLAKE_INIT_CODE,
	MSSQL_INIT_CODE,
	GRAPHQL_INIT_CODE,
	DENO_INIT_CODE_TRIGGER,
	DENO_INIT_CODE_CLEAR,
	PYTHON_INIT_CODE_CLEAR,
	DENO_INIT_CODE_APPROVAL,
	DENO_FAILURE_MODULE_CODE,
	BASH_INIT_CODE,
	POWERSHELL_INIT_CODE
]

export function isInitialCode(content: string): boolean {
	for (const code of ALL_INITIAL_CODE) {
		if (content === code) {
			return true
		}
	}
	return false
}

export function initialCode(
	language: SupportedLanguage,
	kind: Script.kind | undefined,
	subkind: 'pgsql' | 'mysql' | 'flow' | 'script' | 'fetch' | 'docker' | 'powershell' | undefined
): string {
	if (!kind) {
		kind = Script.kind.SCRIPT
	}
	if (language === 'deno') {
		if (kind === 'trigger') {
			return DENO_INIT_CODE_TRIGGER
		} else if (kind === 'script') {
			if (subkind === 'flow') {
				return DENO_INIT_CODE_CLEAR
			} else if (subkind === 'pgsql') {
				return POSTGRES_INIT_CODE
			} else if (subkind === 'mysql') {
				return MYSQL_INIT_CODE
			} else if (subkind === 'fetch') {
				return FETCH_INIT_CODE
			} else {
				return DENO_INIT_CODE
			}
		} else if (kind === 'failure') {
			return DENO_FAILURE_MODULE_CODE
		} else if (kind === 'approval') {
			return DENO_INIT_CODE_APPROVAL
		} else {
			return DENO_INIT_CODE
		}
	} else if (language === 'python3') {
		if (kind === 'trigger') {
			return PYTHON_INIT_CODE_TRIGGER
		} else if (subkind === 'flow') {
			return PYTHON_INIT_CODE_CLEAR
		} else if (kind === 'failure') {
			return PYTHON_FAILURE_MODULE_CODE
		} else {
			return PYTHON_INIT_CODE
		}
	} else if (language == 'bash') {
		if (subkind === 'docker') {
			return DOCKER_INIT_CODE
		} else {
			return BASH_INIT_CODE
		}
	} else if (language == 'powershell') {
		return POWERSHELL_INIT_CODE
	} else if (language == 'nativets') {
		return NATIVETS_INIT_CODE
	} else if (language == 'postgresql') {
		return POSTGRES_INIT_CODE
	} else if (language == 'mysql') {
		return MYSQL_INIT_CODE
	} else if (language == 'bigquery') {
		return BIGQUERY_INIT_CODE
	} else if (language == 'snowflake') {
		return SNOWFLAKE_INIT_CODE
	} else if (language == 'mssql') {
		return MSSQL_INIT_CODE
	} else if (language == 'graphql') {
		return GRAPHQL_INIT_CODE
	} else if (language == 'bun') {
		if (kind === 'approval') {
			return BUN_INIT_CODE_APPROVAL
		}
		if (subkind === 'flow') {
			return BUN_INIT_CODE_CLEAR
		}
		return BUN_INIT_CODE
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

export function getResetCode(
	language: SupportedLanguage,
	kind: Script.kind | undefined,
	subkind: 'pgsql' | 'mysql' | 'flow' | 'script' | 'fetch' | 'docker' | 'powershell' | undefined
) {
	if (language === 'deno') {
		return DENO_INIT_CODE_CLEAR
	} else if (language === 'python3') {
		return PYTHON_INIT_CODE_CLEAR
	} else if (language === 'nativets') {
		return NATIVETS_INIT_CODE_CLEAR
	} else if (language === 'bun') {
		return BUN_INIT_CODE_CLEAR
	} else {
		return initialCode(language, kind, subkind)
	}
}
