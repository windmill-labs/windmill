export const PYTHON_INIT_CODE = `import os
import wmill
from datetime import datetime
# Our webeditor includes a syntax, type checker through a language server running pyright
# and the autoformatter Black in our servers. Use Cmd/Ctrl + S to autoformat the code.
# Beware that the code is only saved when you click Save and not across reload.
# You can however navigate to any steps safely.
"""
The client is used to interact with windmill itself through its standard API.
One can explore the methods available through autocompletion of \`client.XXX\`.
Only the most common methods are included for ease of use. Request more as
feedback if you feel you are missing important ones.
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
    print(f"The env variable at \`g_all/pretty_secret\`: {secret}")
    # interact with the windmill platform to get the version
    version = wmill.get_version()
    # fetch reserved variables as environment variables
    user = os.environ.get("WM_USERNAME")
    # the return value is then parsed and can be retrieved by other scripts conveniently
    return {"version": version, "splitted": name.split(), "user": user}
`
export const DENO_INIT_CODE = `// only do the following import if you require your script to interact with the windmill
// for instance to get a variable or resource
// import * as wmill from 'https://deno.land/x/windmill@v${__pkg__.version}/mod.ts'

export async function main(x: string, y: string = 'default arg') {
	// let x = await wmill.getVariable('u/user/foo');
	// let y = await wmill.getResource('u/user/foo')
	return { foo: x }
}
`
