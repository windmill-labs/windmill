<script lang="ts">
	import { ScriptService, type Script } from '../../gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '../../stores'
	import ScriptBuilder from '../components/ScriptBuilder.svelte'
	import type { Schema } from '../../common'
	import { sendUserToast } from '../../utils'

	// Default
	let schema: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {
			age: {
				default: 42,
				description: '',
				type: 'integer'
			},
			l: {
				default: ['or', 'lists!'],
				description: '',
				type: 'array'
			},
			name: {
				default: 'Nicolas Bourbaki',
				description: '',
				type: 'string'
			},
			obj: {
				default: {
					even: 'dicts'
				},
				description: '',
				type: 'object'
			}
		},
		required: [],
		type: 'object'
	}

	let code = `import os
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

	$: templatePath = $page.url.searchParams.get('template')

	const initialState = $page.url.searchParams.get('state')

	let script: Script =
		initialState != undefined
			? JSON.parse(atob(initialState))
			: {
					hash: '',
					path: '',
					summary: '',
					content: code,
					schema: schema,
					created_by: '',
					created_at: '',
					archived: false,
					deleted: false,
					is_template: false,
					extra_perms: {}
			  }

	async function loadTemplate(): Promise<void> {
		if (templatePath) {
			try {
				const template = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: templatePath
				})
				script.summary = `Copy of ${template.summary}`
				script.description = template.description
				script.content = template.content
				script.schema = template.schema
				sendUserToast('Code & arguments have been loaded from template.')
			} catch (err) {
				sendUserToast(`Could not load template: ${err}`, true)
			}
		}
	}

	$: {
		if ($workspaceStore) {
			loadTemplate()
		}
	}
</script>

<ScriptBuilder {script} />
