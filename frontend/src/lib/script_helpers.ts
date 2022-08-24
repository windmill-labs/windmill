export const PYTHON_INIT_CODE = `import os
import wmill
from datetime import datetime

"""
Use Cmd/Ctrl + S to autoformat the code.
The client is used to interact with windmill itself through its standard API.
One can explore the methods available through autocompletion of \`wmill.XXX\`.
"""

def main(name: str = "Nicolas Bourbaki",
         age: int = 42,
         obj: dict = {"even": "dicts"},
         l: list = ["or", "lists!"],
         file_: bytes = bytes(0),
         dtime: datetime = datetime.now()):
    """A main function is required for the script to be able to accept arguments.
    Types are recommended."""
    print(f"Hello World and a warm welcome especially to {name}")
    print("and its acolytes..", age, obj, l, len(file_), dtime)
    # retrieve variables, including secrets by querying the windmill platform.
    # secret fetching is audited by windmill.
    secret = wmill.get_variable("g/all/pretty_secret")
    print(f"The env variable at \`g/all/pretty_secret\`: {secret}")
    # interact with the windmill platform to get the version
    version = wmill.get_version()
    # fetch reserved variables as environment variables
    user = os.environ.get("WM_USERNAME")
    # the return value is then parsed and can be retrieved by other scripts conveniently
    return {"version": version, "splitted": name.split(), "user": user}
`
export const DENO_INIT_CODE = `// reload the smart assistant on the top right if it dies to get autocompletion and syntax highlighting
// (Ctrl+space to cache dependencies on imports hover).
// to import most npm packages without deno.land, use esm:
// import { toWords } from "https://esm.sh/number-to-words"
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

export const DENO_INIT_CODE_CLEAR = `// import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main() {
  return
}
`

export const PYTHON_INIT_CODE_CLEAR = `#import wmill

def main():
  return
`

export const POSTGRES_INIT_CODE = `import {
  pgSql,
  type Resource,
} from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts";

//PG parameterized statement. No SQL injection is possible.
export async function main(
  db: Resource<"postgresql"> = "$res:g/all/demodb",
  key: string,
  value: string,
) {
  const query = await pgSql(
    db,
  )\`INSERT INTO demo VALUES (\${key}, \${value}) RETURNING *\`;
  return query.rows;
}`

export const DENO_INIT_CODE_TRIGGER = `import * as wmill from "https://deno.land/x/windmill@v${__pkg__.version}/mod.ts"

export async function main() {

    // A common trigger script would follow this pattern:
    // 1. Get the last saved state
    // const state = wmill.getInternalState()
    // 2. Get the actual state from the external service
    // const newState = await (await fetch('https://hacker-news.firebaseio.com/v0/topstories.json')).json()
    // 3. Compare the two states and update the internal state
    // wmill.setInternalState(newState)
    // 4. Return the new rows
    // return range from (state to newState)

	return [1,2,3]

    // In subsequent scripts, you may refer to each row/value returned by the trigger script using
    // 'flow_input._value'
}
`

export function initialCode(language: 'deno' | 'python3', type: 'trigger' | 'flow' | 'pgsql' | undefined): string {
  if (language === 'deno') {
    if (type === 'trigger') {
      return DENO_INIT_CODE_TRIGGER
    } else if (type === 'flow') {
      return DENO_INIT_CODE_CLEAR
    } else if (type === 'pgsql') {
      return POSTGRES_INIT_CODE
    } else {
      return DENO_INIT_CODE
    }
  } else {
    if (type === 'flow') {
      return PYTHON_INIT_CODE_CLEAR
    } else {
      return PYTHON_INIT_CODE
    }
  }
}
