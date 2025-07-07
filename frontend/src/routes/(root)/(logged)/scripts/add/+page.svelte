<script lang="ts">
	import { type NewScript, ScriptService, type Script } from '$lib/gen'

	import { page } from '$app/stores'
	import { defaultScripts, initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { Schema } from '$lib/common'
	import { decodeState, emptySchema, emptyString } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { replaceState } from '$app/navigation'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'

	// Default
	let schema: Schema = emptySchema()

	const templatePath = $page.url.searchParams.get('template')
	const hubPath = $page.url.searchParams.get('hub')
	const showMeta = /true|1/i.test($page.url.searchParams.get('show_meta') ?? '0')

	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	const path = $page.url.searchParams.get('path')

	const initialState = $page.url.hash != '' ? $page.url.hash.slice(1) : undefined

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	function decodeStateAndHandleError(state) {
		try {
			const decoded = decodeState(state)
			return decoded
		} catch (e) {
			console.error('Error decoding state', e)
			return defaultScript()
		}
	}

	function defaultScript() {
		return {
			hash: '',
			path: path ?? '',
			summary: '',
			content: '',
			schema: schema,
			is_template: false,
			extra_perms: {},
			language:
				$defaultScripts?.order?.filter(
					(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x)
				)?.[0] ?? 'bun',
			kind: 'script'
		}
	}

	let script: NewScript & { draft_triggers: Trigger[] } = $state(
		!path && initialState != undefined ? decodeStateAndHandleError(initialState) : defaultScript()
	)

	async function loadTemplate(): Promise<void> {
		if (templatePath) {
			const template = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})

			// Only copy the summary if it's not empty
			script.summary = !emptyString(template.summary) ? `Copy of ${template.summary}` : ''
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
			script.content = replaceScriptPlaceholderWithItsValues(hubPath, content)
			script.summary = summary ?? ''
			script.language = language as Script['language']
			scriptBuilder?.setCode(script.content)
		}
	}

	loadHub()

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadTemplate())
		}
	})
</script>

<ScriptBuilder
	{initialArgs}
	bind:this={scriptBuilder}
	lockedLanguage={templatePath != null || hubPath != null}
	onDeploy={(e) => {
		goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
	}}
	onSaveInitial={(e) => {
		goto(`/scripts/edit/${e.path}`)
	}}
	searchParams={$page.url.searchParams}
	{script}
	{showMeta}
	replaceStateFn={(path) => replaceState(path, $page.state)}
>
	<UnsavedConfirmationModal
		getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
	/>
</ScriptBuilder>
