<script lang="ts">
	import { ScriptService, type Script } from '../../gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '../../stores'
	import ScriptBuilder from '../components/ScriptBuilder.svelte'
	import type { Schema } from '../../common'
	import { emptySchema, sendUserToast } from '../../utils'

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
