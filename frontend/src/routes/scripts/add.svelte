<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { Schema } from '$lib/common'
	import { emptySchema, sendUserToast } from '$lib/utils'

	// Default
	let schema: Schema = emptySchema()

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
					language: 'python3'
			  }

	async function loadTemplate(): Promise<void> {
		if (templatePath) {
			const template = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			script.summary = `Copy of ${template.summary}`
			script.description = template.description
			script.content = template.content
			script.schema = template.schema
			sendUserToast('Code & arguments have been loaded from template.')
		}
	}

	$: {
		if ($workspaceStore) {
			loadTemplate()
		}
	}
</script>

<ScriptBuilder {script} />
