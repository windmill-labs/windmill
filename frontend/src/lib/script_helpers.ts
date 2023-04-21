import type { Script } from './gen'

import PYTHON_INIT_CODE from '$lib/init_scripts/python_init_code'
import PYTHON_INIT_CODE_CLEAR from '$lib/init_scripts/python_init_code_clear'
import PYTHON_INIT_CODE_TRIGGER from '$lib/init_scripts/python_init_code_trigger'
import PYTHON_FAILURE_MODULE_CODE from '$lib/init_scripts/python_failure_module'

export {
	PYTHON_INIT_CODE,
	PYTHON_INIT_CODE_CLEAR,
	PYTHON_INIT_CODE_TRIGGER,
	PYTHON_FAILURE_MODULE_CODE
}

export const DENO_INIT_CODE = `// Ctrl/CMD+. to cache dependencies on imports hover, Ctrl/CMD+S to format.

// import { toWords } from "npm:number-to-words@1"
// import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main(
  a: number,
  b: "my" | "enum",
  //c: wmill.Resource<'postgresql'>,
  d = "inferred type string from default arg",
  e = { nested: "object" },
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

export const DENO_INIT_CODE_CLEAR = `// import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main(x: string) {
  return x
}
`

export const DENO_FAILURE_MODULE_CODE = `// flow is considered recovered and a success unless an exception is thrown

export async function main(message: string, name: string) {
  const flow_id = Deno.env.get("WM_FLOW_JOB_ID")
  console.log("message", message)
  console.log("name",name)
  return { message, flow_id }
}
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

	/**
	// The following code accepts raw queries. The code above is recommended because it uses SQL prepared statement.
	import { pgClient, type Resource, type Sql } from "https://deno.land/x/windmill@v1.88.1/mod.ts";

	export async function main(
		db: Resource<"postgresql">,
		query: Sql = "SELECT * FROM demo;",
	) {
		if(!query) {
			throw Error('Query must not be empty.')
		}
		const { rows } = await pgClient(db).queryObject(query);
		return rows;
	}
   */
	
}`

export const MYSQL_INIT_CODE = `import {
  mySql,
  type Resource
} from "https://deno.land/x/windmill@v${__pkg__.version}/mysql.ts";

// MySQL parameterized statement. No SQL injection is possible.
export async function main(
  db: Resource<"mysql">,
  key: number,
  value: string,
) {
  const query = await mySql(
    db
  )\`INSERT INTO demo VALUES (\${key}, \${value})\`;
  return query.rows;
}`

export const FETCH_INIT_CODE = `export async function main(
	url: string | undefined,
	method: string = 'GET',
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

export const BASH_INIT_CODE = `# arguments of the form X="$I" are parsed as parameters X of type string
msg="$1"
dflt="\${2:-default value}"

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

const ALL_INITIAL_CODE = [
	PYTHON_INIT_CODE,
	PYTHON_INIT_CODE_TRIGGER,
	DENO_INIT_CODE,
	POSTGRES_INIT_CODE,
	DENO_INIT_CODE_TRIGGER,
	DENO_INIT_CODE_CLEAR,
	PYTHON_INIT_CODE_CLEAR,
	DENO_INIT_CODE_APPROVAL,
	DENO_FAILURE_MODULE_CODE
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
	language: 'deno' | 'python3' | 'go' | 'bash',
	kind: Script.kind,
	subkind: 'pgsql' | 'mysql' | 'flow' | 'script' | 'fetch' | undefined
): string {
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
