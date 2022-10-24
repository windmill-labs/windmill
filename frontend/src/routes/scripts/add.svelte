<script context="module">
	export function load() {
		return {
			stuff: { title: `New Script` }
		}
	}
</script>

<script lang="ts">
	import { Script, ScriptService } from '$lib/gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { Schema } from '$lib/common'
	import { decodeState, emptySchema, getScriptByPath, sendUserToast } from '$lib/utils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	// Default
	let schema: Schema = emptySchema()

	const templatePath = $page.url.searchParams.get('template')
	const hubPath = $page.url.searchParams.get('hub')

	const initialState = $page.url.searchParams.get('state')

	let script: Script =
		initialState != undefined
			? decodeState(initialState)
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
					language: 'deno',
					kind: Script.kind.SCRIPT
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
			script.language = template.language
			sendUserToast('Code & arguments have been loaded from template.')
		}
	}

	async function loadHub(): Promise<void> {
		if (hubPath) {
			const template = await getScriptByPath(hubPath)
			script.description = `Fork of ${hubPath}`
			script.content = template.content
			script.language = Script.language.DENO
			sendUserToast(`Code has been loaded from hub script ${hubPath}.`)
		}
	}

	loadHub()

	$: {
		if ($workspaceStore) {
			loadTemplate()
		}
	}
	$dirtyStore = true
</script>

<ScriptBuilder {script} />
