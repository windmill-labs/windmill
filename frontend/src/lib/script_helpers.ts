import { type Script } from './gen'

import type { SupportedLanguage } from './common'

const PYTHON_FAILURE_MODULE_CODE = `import os

def main(message: str, name: str, step_id: str):
    flow_id = os.environ.get("WM_ROOT_FLOW_JOB_ID")
    print("message", message)
    print("name", name)
    print("step_id", step_id)
    return { "message": message, "flow_id": flow_id, "step_id": step_id, "recover": False }`

const PYTHON_INIT_CODE_CLEAR = `# import wmill


def main(x: str):
    return x`

const PYTHON_INIT_CODE_TRIGGER = `import wmill


def main():
    # A common trigger script would follow this pattern:
    # 1. Get the last saved state
    # state = wmill.get_state()
    # 2. Get the actual state from the external service
    # newState = ...
    # 3. Compare the two states and update the internal state
    # wmill.setState(newState)
    # 4. Return the new rows
    # return range from (state to newState)
    return [1, 2, 3]`

const PYTHON_INIT_CODE = `import os
import wmill

# You can import any PyPi package. 
# See here for more info: https://www.windmill.dev/docs/advanced/dependencies_in_python

# you can use typed resources by doing a type alias to dict
#postgresql = dict

def main(
    no_default: str,
    #db: postgresql,
    name="Nicolas Bourbaki",
    age=42,
    obj: dict = {"even": "dicts"},
    l: list = ["or", "lists!"],
    file_: bytes = bytes(0),
):

    print(f"Hello World and a warm welcome especially to {name}")
    print("and its acolytes..", age, obj, l, len(file_))

    # retrieve variables, resources, states using the wmill client
    try:
        secret = wmill.get_variable("f/examples/secret")
    except:
        secret = "No secret yet at f/examples/secret !"
    print(f"The variable at \`f/examples/secret\`: {secret}")

    # Get last state of this script execution by the same trigger/user
    last_state = wmill.get_state()
    new_state = {"foo": 42} if last_state is None else last_state
    new_state["foo"] += 1
    wmill.set_state(new_state)

    # fetch context variables
    user = os.environ.get("WM_USERNAME")

    # return value is converted to JSON
    return {"splitted": name.split(), "user": user, "state": new_state}`

const NATIVETS_INIT_CODE = `// Fetch-only script, no imports allowed (except windmill) but benefits from a dedicated highly efficient runtime
//import * as wmill from './windmill.ts'

export async function main(example_input: number = 3) {
  // "3" is the default value of example_input, it can be overriden with code or using the UI
  const res = await fetch(\`https://jsonplaceholder.typicode.com/todos/\${example_input}\`, {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
`

const BUNNATIVE_INIT_CODE = `//native
//you can add proxy support using //proxy http(s)://host:port

// native scripts are bun scripts that are executed on native workers and can be parallelized
// only fetch is allowed, but imports will work as long as they also use only fetch and the standard lib

//import * as wmill from "windmill-client"

export async function main(example_input: number = 3) {
  // "3" is the default value of example_input, it can be overriden with code or using the UI
  const res = await fetch(\`https://jsonplaceholder.typicode.com/todos/\${example_input}\`, {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
`

const NATIVETS_INIT_CODE_CLEAR = `// Fetch-only script, no imports allowed (except windmill) but benefits from a dedicated highly efficient runtime
//import * as wmill from './windmill.ts'

export async function main() {
  const res = await fetch("https://jsonplaceholder.typicode.com/todos/1", {
    headers: { "Content-Type": "application/json" },
  });
  return res.json();
}
`

const DENO_INIT_CODE = `// Ctrl/CMD+. to cache dependencies on imports hover.

// Deno uses "npm:" prefix to import from npm (https://deno.land/manual@v1.36.3/node/npm_specifiers)
// import * as wmill from "npm:windmill-client@${__pkg__.version}"

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

const BUN_INIT_CODE = `// there are multiple modes to add as header: //nobundling //native //npm //nodejs
// https://www.windmill.dev/docs/getting_started/scripts_quickstart/typescript#modes

// import { toWords } from "number-to-words@1"
import * as wmill from "windmill-client"

// fill the type, or use the +Resource type to get a type-safe reference to a resource
// type Postgresql = object


export async function main(
  a: number,
  b: "my" | "enum",
  //c: Postgresql,
  //d: wmill.S3Object, // https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files 
  //d: DynSelect_foo, // https://www.windmill.dev/docs/core_concepts/json_schema_and_parsing#dynamic-select
  e = "inferred type string from default arg",
  f = { nested: "object" },
  g: {
    label: "Variant 1",
    foo: string
  } | {
    label: "Variant 2",
    bar: number
  }
) {
  // let x = await wmill.getVariable('u/user/foo')
  return { foo: a };
}
`

const GO_INIT_CODE = `package inner

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

const GO_FAILURE_MODULE_CODE = `package inner

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

const DENO_INIT_CODE_CLEAR = `// import * as wmill from "npm:windmill-client@${__pkg__.version}"

export async function main(x: string) {
  return x
}
`

const BUN_INIT_CODE_CLEAR = `// import * as wmill from "windmill-client"

export async function main(x: string) {
  return x
}
`

const DENO_FAILURE_MODULE_CODE = `
export async function main(message: string, name: string, step_id: string) {
  const flow_id = Deno.env.get("WM_ROOT_FLOW_JOB_ID")
  console.log("message", message)
  console.log("name",name)
  console.log("step_id", step_id)
  return { message, flow_id, step_id, recover: false }
}
`

const BUN_FAILURE_MODULE_CODE = `
export async function main(message: string, name: string, step_id: string) {
  const flow_id = process.env.WM_ROOT_FLOW_JOB_ID
  console.log("message", message)
  console.log("name",name)
  console.log("step_id", step_id)
  return { message, flow_id, step_id, recover: false }
}
`

const POSTGRES_INIT_CODE = `-- to pin the database use '-- database f/your/path'
-- $1 name1 = default arg
-- $2 name2
-- $3 name3
-- $4 name4
INSERT INTO demo VALUES (\$1::TEXT, \$2::INT, \$3::TEXT[]) RETURNING *;
UPDATE demo SET col2 = \$4::INT WHERE col2 = \$2::INT;
`

const MYSQL_INIT_CODE = `-- to pin the database use '-- database f/your/path'
-- :name1 (text) = default arg
-- :name2 (int)
-- :name3 (int)
INSERT INTO demo VALUES (:name1, :name2);
UPDATE demo SET col2 = :name3 WHERE col2 = :name2;
`

const BIGQUERY_INIT_CODE = `-- to pin the database use '-- database f/your/path'
-- @name1 (string) = default arg
-- @name2 (integer)
-- @name3 (string[])
-- @name4 (integer)
INSERT INTO \`demodb.demo\` VALUES (@name1, @name2, @name3);
UPDATE \`demodb.demo\` SET col2 = @name4 WHERE col2 = @name2;
`

const SNOWFLAKE_INIT_CODE = `-- to pin the database use '-- database f/your/path'
-- ? name1 (varchar) = default arg
-- ? name2 (int)
INSERT INTO demo VALUES (?, ?);
-- ? name3 (int)
-- ? name2 (int)
UPDATE demo SET col2 = ? WHERE col2 = ?;
`

const MSSQL_INIT_CODE = `-- return_last_result
-- to pin the database use '-- database f/your/path'
-- @p1 name1 (varchar) = default arg
-- @p2 name2 (int)
-- @p3 name3 (int)
INSERT INTO demo VALUES (@p1, @p2);
UPDATE demo SET col2 = @p3 WHERE col2 = @p2;
`

const GRAPHQL_INIT_CODE = `query($name4: String, $name2: Int, $name3: [String]) {
	demo(name1: $name1, name2: $name2, name3: $name3) {
		name1,
		name2,
		name3
	}
}
`

const PHP_INIT_CODE = `<?php

// remove the first // of the following lines to specify packages to install using composer
// // require:
// // monolog/monolog@3.6.0
// // stripe/stripe-php

function main(
  // Postgresql $a,
  // array $b,
  // object $c,
  int $d = 123,
  string $e = "default value",
  float $f = 3.5,
  bool $g = true,
) {
  return $d;
}
`

const RUST_INIT_CODE = `//! Add dependencies in the following partial Cargo.toml manifest
//!
//! \`\`\`cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! rand = "0.7.2"
//! \`\`\`
//!
//! Note that serde is used by default with the \`derive\` feature.
//! You can still reimport it if you need additional features.

use anyhow::anyhow;
use rand::seq::SliceRandom;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Ret {
    msg: String,
    number: i8,
}

fn main(who_to_greet: String, numbers: Vec<i8>) -> anyhow::Result<Ret> {
    println!(
        "Person to greet: {} -  numbers to choose: {:?}",
        who_to_greet, numbers
    );
    Ok(Ret {
        msg: format!("Greetings {}!", who_to_greet),
        number: *numbers
            .choose(&mut rand::thread_rng())
            .ok_or(anyhow!("There should be some numbers to choose from"))?,
    })
}
`

const FETCH_INIT_CODE = `export async function main(
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

const BASH_INIT_CODE = `# shellcheck shell=bash
# arguments of the form X="$I" are parsed as parameters X of type string
msg="$1"
dflt="\${2:-default value}"

# the last line of the stdout is the return value
# unless you write json to './result.json' or a string to './result.out'
echo "Hello $msg"
`

const DENO_INIT_CODE_TRIGGER = `import * as wmill from "npm:windmill-client@${__pkg__.version}"

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

const BUN_INIT_CODE_TRIGGER = `import * as wmill from "windmill-client"

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

const GO_INIT_CODE_TRIGGER = `package inner

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

const DENO_INIT_CODE_APPROVAL = `import * as wmill from "npm:windmill-client@^1.158.2"

export async function main(approver?: string) {
  const urls = await wmill.getResumeUrls(approver)
  // send the urls to their intended recipients

  return {
    // if the resumeUrls are part of the response, they will be available to any persons having access
    // to the run page and allowed to be approved from there, even from non owners of the flow
    // self-approval is disableable in the suspend options
    	...urls,

    // to have prompts (self-approvable steps), clude instead the resume url in the returned payload of the step
    // the UX will automatically adapt and show the prompt to the operator when running the flow. e.g:
    // resume: urls['resume'],

		default_args: {},
		enums: {},
		description: undefined
		// supports all formats from rich display rendering such as simple strings,
		// but also markdown, html, images, tables, maps, render_all, etc...
		// https://www.windmill.dev/docs/core_concepts/rich_display_rendering
  }
}

// add a form in Advanced - Suspend
// all on approval steps: https://www.windmill.dev/docs/flows/flow_approval`

const BUN_INIT_CODE_APPROVAL = `import * as wmill from "windmill-client@^1.158.2"

export async function main(approver?: string) {
  const urls = await wmill.getResumeUrls(approver)
  // send the urls to their intended recipients

  return {
    // if the resumeUrls are part of the response, they will be available to any persons having access
    // to the run page and allowed to be approved from there, even from non owners of the flow
    // self-approval is disableable in the suspend options
    	...urls,

    // to have prompts (self-approvable steps), clude instead the resume url in the returned payload of the step
    // the UX will automatically adapt and show the prompt to the operator when running the flow. e.g:
    // resume: urls['resume'],

		default_args: {},
		enums: {},
		description: undefined
		// supports all formats from rich display rendering such as simple strings,
		// but also markdown, html, images, tables, maps, render_all, etc...
		// https://www.windmill.dev/docs/core_concepts/rich_display_rendering
  }
}

// add a form in Advanced - Suspend
// all on approval steps: https://www.windmill.dev/docs/flows/flow_approval`

const BUN_PREPROCESSOR_MODULE_CODE = `
export async function preprocessor(
	wm_trigger: {
		kind: 'http' | 'email' | 'webhook' | 'websocket',
		http?: {
			route: string // The route path, e.g. "/users/:id"
			path: string // The actual path called, e.g. "/users/123"
			method: string
			params: Record<string, string>
			query: Record<string, string>
			headers: Record<string, string>
		}
	},
	/* your other args */ 
) {
	return {
		// return the args to be passed to the flow
	}
}
`

const DENO_PREPROCESSOR_MODULE_CODE = `
export async function preprocessor(
	wm_trigger: {
		kind: 'http' | 'email' | 'wehbook' | 'websocket',
		http?: {
			route: string // The route path, e.g. "/users/:id"
			path: string // The actual path called, e.g. "/users/123"
			method: string
			params: Record<string, string>
			query: Record<string, string>
			headers: Record<string, string>
		}
	},
	/* your other args */ 
) {
	return {
		// return the args to be passed to the flow
	}
}
`

const PYTHON_INIT_CODE_APPROVAL = `import wmill

def main():
  urls = wmill.get_resume_urls()
  # send the urls to their intended recipients

  return {
    # if the get_resume_urls are part of the response, they will be available to any persons having access
    # to the run page and allowed to be approved from there, even from non owners of the flow
    # self-approval is disableable in the suspend options
    **urls,

    # to have prompts (self-approvable steps), clude instead the resume url in the returned payload of the step
    # the UX will automatically adapt and show the prompt to the operator when running the flow. e.g:
    # "resume": urls["resume"],

    "default_args": {},
    "enums": {},
    "description": None,
    # supports all formats from rich display rendering such as simple strings,
    # but also markdown, html, images, tables, maps, render_all, etc...
    # https://www.windmill.dev/docs/core_concepts/rich_display_rendering
  }

# add a form in Advanced - Suspend
# all on approval steps: https://www.windmill.dev/docs/flows/flow_approval`

const PYTHON_PREPROCESSOR_MODULE_CODE = `from typing import TypedDict, Literal

class Http(TypedDict):
	route: str # The route path, e.g. "/users/:id"
	path: str # The actual path called, e.g. "/users/123"
	method: str
	params: dict[str, str]
	query: dict[str, str]
	headers: dict[str, str]

class WmTrigger(TypedDict):
    kind: Literal["http", "email", "webhook", "websocket"]
    http: Http | None

def preprocessor(
	wm_trigger: WmTrigger,
	# your other args
):
	return {
		# return the args to be passed to the flow
	}
`

const DOCKER_INIT_CODE = `# shellcheck shell=bash
# Bash script that calls docker as a client to the host daemon
# See documentation: https://www.windmill.dev/docs/advanced/docker
msg="\${1:-world}"

IMAGE="alpine:latest"
COMMAND="/bin/echo Hello $msg"

# ensure that the image is up-to-date
docker pull $IMAGE
docker run --rm $IMAGE $COMMAND
`

const POWERSHELL_INIT_CODE = `param($Msg, $Dflt = "default value", [int]$Nb = 3)

# Import-Module MyModule

# Import-Module WindmillClient
# Connect-Windmill
# Get-WindmillVariable -Path 'u/user/foo'

# the last line of the stdout is the return value
Write-Output "Hello $Msg"`

const ANSIBLE_PLAYBOOK_INIT_CODE = `---
inventory:
  - resource_type: ansible_inventory
    # You can pin an inventory to this script by hardcoding the resource path:
    # resource: u/user/your_resource

# File resources will be written in the relative \`target\` location before
# running the playbook
# file_resources:
  # - resource: u/user/fabulous_jinja_template
  #   target:  ./config_template.j2

# Define the arguments of the windmill script
extra_vars:
  world_qualifier:
    type: string

dependencies:
  galaxy:
    collections:
      - name: community.general
      - name: community.vmware
  python:
    - jmespath
---
- name: Echo
  hosts: 127.0.0.1
  connection: local
  vars:
    my_result:
      a: 2
      b: true
      c: "Hello"

  tasks:
  - name: Print debug message
    debug:
      msg: "Hello, {{world_qualifier}} world!"
  - name: Write variable my_result to result.json
    delegate_to: localhost
    copy:
      content: "{{ my_result | to_json }}"
      dest: result.json
`
export const INITIAL_CODE = {
	bun: {
		script: BUN_INIT_CODE,
		trigger: BUN_INIT_CODE_TRIGGER,
		approval: BUN_INIT_CODE_APPROVAL,
		failure: BUN_FAILURE_MODULE_CODE,
		preprocessor: BUN_PREPROCESSOR_MODULE_CODE,
		clear: BUN_INIT_CODE_CLEAR
	},
	python3: {
		script: PYTHON_INIT_CODE,
		trigger: PYTHON_INIT_CODE_TRIGGER,
		approval: PYTHON_INIT_CODE_APPROVAL,
		failure: PYTHON_FAILURE_MODULE_CODE,
		preprocessor: PYTHON_PREPROCESSOR_MODULE_CODE,
		clear: PYTHON_INIT_CODE_CLEAR
	},
	deno: {
		script: DENO_INIT_CODE,
		trigger: DENO_INIT_CODE_TRIGGER,
		approval: DENO_INIT_CODE_APPROVAL,
		failure: DENO_FAILURE_MODULE_CODE,
		preprocessor: DENO_PREPROCESSOR_MODULE_CODE,
		fetch: FETCH_INIT_CODE,
		clear: DENO_INIT_CODE_CLEAR
	},
	go: {
		script: GO_INIT_CODE,
		trigger: GO_INIT_CODE_TRIGGER,
		failure: GO_FAILURE_MODULE_CODE
	},
	bash: {
		script: BASH_INIT_CODE
	},
	powershell: {
		script: POWERSHELL_INIT_CODE
	},
	nativets: {
		script: NATIVETS_INIT_CODE
	},
	postgresql: {
		script: POSTGRES_INIT_CODE
	},
	mysql: {
		script: MYSQL_INIT_CODE
	},
	bigquery: {
		script: BIGQUERY_INIT_CODE
	},
	snowflake: {
		script: SNOWFLAKE_INIT_CODE
	},
	mssql: {
		script: MSSQL_INIT_CODE
	},
	graphql: {
		script: GRAPHQL_INIT_CODE
	},
	php: {
		script: PHP_INIT_CODE
	},
	rust: {
		script: RUST_INIT_CODE
	},
	ansible: {
		script: ANSIBLE_PLAYBOOK_INIT_CODE
	},
	docker: {
		script: DOCKER_INIT_CODE
	},
	bunnative: {
		script: BUNNATIVE_INIT_CODE
	}
}

export function isInitialCode(content: string): boolean {
	Object.values(INITIAL_CODE).forEach((lang) => {
		Object.values(lang).forEach((code) => {
			if (content === code) {
				return true
			}
		})
	})

	return false
}

export function initialCode(
	language: SupportedLanguage | 'bunnative' | undefined,
	kind: Script['kind'] | undefined,
	subkind:
		| 'pgsql'
		| 'mysql'
		| 'flow'
		| 'script'
		| 'fetch'
		| 'docker'
		| 'powershell'
		| 'bunnative'
		| 'preprocessor'
		| undefined
): string {
	if (!kind) {
		kind = 'script'
	}
	if (language === 'deno') {
		if (kind === 'trigger') {
			return INITIAL_CODE.deno.trigger
		} else if (kind === 'script') {
			if (subkind === 'flow') {
				return INITIAL_CODE.deno.clear
			} else if (subkind === 'pgsql') {
				return INITIAL_CODE.postgresql.script
			} else if (subkind === 'mysql') {
				return INITIAL_CODE.mysql.script
			} else if (subkind === 'fetch') {
				return INITIAL_CODE.deno.fetch
			} else if (subkind === 'preprocessor') {
				return INITIAL_CODE.deno.preprocessor
			} else {
				return INITIAL_CODE.deno.script
			}
		} else if (kind === 'failure') {
			return INITIAL_CODE.deno.failure
		} else if (kind === 'approval') {
			return INITIAL_CODE.deno.approval
		} else {
			return INITIAL_CODE.deno.script
		}
	} else if (language === 'python3') {
		if (kind === 'trigger') {
			return INITIAL_CODE.python3.trigger
		} else if (kind === 'approval') {
			return INITIAL_CODE.python3.approval
		} else if (kind === 'failure') {
			return INITIAL_CODE.python3.failure
		} else if (subkind === 'flow') {
			return INITIAL_CODE.python3.clear
		} else if (subkind === 'preprocessor') {
			return INITIAL_CODE.python3.preprocessor
		} else {
			return INITIAL_CODE.python3.script
		}
	} else if (language == 'bash') {
		if (subkind === 'docker') {
			return INITIAL_CODE.docker.script
		} else {
			return INITIAL_CODE.bash.script
		}
	} else if (language == 'powershell') {
		return INITIAL_CODE.powershell.script
	} else if (language == 'nativets') {
		return INITIAL_CODE.nativets.script
	} else if (language == 'postgresql') {
		return INITIAL_CODE.postgresql.script
	} else if (language == 'mysql') {
		return INITIAL_CODE.mysql.script
	} else if (language == 'bigquery') {
		return INITIAL_CODE.bigquery.script
	} else if (language == 'snowflake') {
		return INITIAL_CODE.snowflake.script
	} else if (language == 'mssql') {
		return INITIAL_CODE.mssql.script
	} else if (language == 'graphql') {
		return INITIAL_CODE.graphql.script
	} else if (language == 'php') {
		return INITIAL_CODE.php.script
	} else if (language == 'rust') {
		return INITIAL_CODE.rust.script
	} else if (language == 'ansible') {
		return INITIAL_CODE.ansible.script
	} else if (language == 'bun' || language == 'bunnative') {
		if (kind == 'trigger') {
			return INITIAL_CODE.bun.trigger
		} else if (language == 'bunnative' || subkind === 'bunnative') {
			return INITIAL_CODE.bunnative.script
		} else if (kind === 'approval') {
			return INITIAL_CODE.bun.approval
		} else if (kind === 'failure') {
			return INITIAL_CODE.bun.failure
		} else if (subkind === 'preprocessor') {
			return INITIAL_CODE.bun.preprocessor
		} else if (subkind === 'flow') {
			return INITIAL_CODE.bun.clear
		}

		return INITIAL_CODE.bun.script
	} else {
		if (kind === 'failure') {
			return INITIAL_CODE.go.failure
		} else if (kind === 'trigger') {
			return INITIAL_CODE.go.trigger
		} else {
			return INITIAL_CODE.go.script
		}
	}
}

export function getResetCode(
	language: SupportedLanguage | 'bunnative' | undefined,
	kind: Script['kind'] | undefined,
	subkind:
		| 'pgsql'
		| 'mysql'
		| 'flow'
		| 'script'
		| 'fetch'
		| 'docker'
		| 'powershell'
		| 'bunnative'
		| undefined
) {
	if (language === 'deno') {
		return DENO_INIT_CODE_CLEAR
	} else if (language === 'python3') {
		return PYTHON_INIT_CODE_CLEAR
	} else if (language === 'nativets') {
		return NATIVETS_INIT_CODE_CLEAR
	} else if (language === 'bun') {
		return BUN_INIT_CODE_CLEAR
	} else if (language === 'bunnative') {
		return BUNNATIVE_INIT_CODE
	} else {
		return initialCode(language, kind, subkind)
	}
}
