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

	$: templatePath = $page.url.searchParams.get('template')

	const initialState = $page.url.searchParams.get('state')

	let script: Script =
		initialState != undefined
			? JSON.parse(atob(initialState))
			: {
					hash: '',
					path: '',
					summary: '',
					content: '',
					schema: schema,
					created_by: '',
					created_at: '',
					archived: false,
					deleted: false,
					is_template: false,
					extra_perms: {},
					language: 'deno'
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
