<script lang="ts">
	import { NewScript, Script, ScriptService } from '$lib/gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { Schema } from '$lib/common'
	import { decodeState, emptySchema } from '$lib/utils'

	// Default
	let schema: Schema = emptySchema()

	const templatePath = $page.url.searchParams.get('template')
	const hubPath = $page.url.searchParams.get('hub')
	const showMeta = /true|1/i.test($page.url.searchParams.get('show_meta') ?? '0')

	const path = $page.url.searchParams.get('path')

	const initialState = $page.url.hash != '' ? $page.url.hash.slice(1) : undefined

	let scriptBuilder: ScriptBuilder | undefined = undefined

	let script: NewScript =
		!path && initialState != undefined
			? decodeState(initialState)
			: {
					hash: '',
					path: path ?? '',
					summary: '',
					content: '',
					schema: schema,
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
			scriptBuilder?.setCode(script.content)
		}
	}

	async function loadHub(): Promise<void> {
		if (hubPath) {
			const { content, language, summary } = await ScriptService.getHubScriptByPath({
				path: hubPath
			})
			script.description = `Fork of ${hubPath}`
			script.content = content
			script.summary = summary ?? ''
			script.language = language as Script.language
			scriptBuilder?.setCode(script.content)
		}
	}

	loadHub()

	$: {
		if ($workspaceStore) {
			loadTemplate()
		}
	}
</script>

<ScriptBuilder
	bind:this={scriptBuilder}
	lockedLanguage={templatePath != null || hubPath != null}
	{script}
	{showMeta}
/>
