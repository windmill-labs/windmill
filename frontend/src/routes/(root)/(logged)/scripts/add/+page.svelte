<script lang="ts">
	import { type NewScript, ScriptService, type ScriptLang } from '$lib/gen'

	import { page } from '$app/stores'
	import { defaultScripts, initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { Schema } from '$lib/common'
	import { decodeState, emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { replaceState } from '$app/navigation'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import ScriptEditorSkeleton from '$lib/components/ScriptEditorSkeleton.svelte'

	type Script = NewScript & {
		draft_triggers?: Trigger[]
		hash?: string
		extra_perms?: Record<string, any>
	}

	// Default
	let schema: Schema = emptySchema()

	const templatePath = $page.url.searchParams.get('template')
	const hubPath = $page.url.searchParams.get('hub')
	const showMeta = /true|1/i.test($page.url.searchParams.get('show_meta') ?? '0')
	const urlArgs = $page.url.searchParams.get('initial_args')

	let initialArgs = urlArgs ? decodeState(urlArgs) : (get(initialArgsStore) ?? {})
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

	function defaultScript(): Script {
		return {
			hash: '',
			path: path ?? '',
			summary: '',
			content: '',
			description: '',
			schema: schema,
			is_template: false,
			extra_perms: {},
			language: ($defaultScripts?.order?.filter(
				(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x)
			)?.[0] ?? 'bun') as ScriptLang,
			kind: 'script'
		}
	}

	let script: Script | undefined = $state(
		templatePath || hubPath
			? undefined
			: !path && initialState != undefined
				? decodeStateAndHandleError(initialState)
				: defaultScript()
	)

	async function loadTemplate(): Promise<void> {
		if (templatePath) {
			try {
				const template = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: templatePath
				})
				script = {
					...defaultScript(),
					summary: !emptyString(template.summary) ? `Copy of ${template.summary}` : '',
					description: template.description,
					content: template.content,
					schema: template.schema,
					language: template.language,
					path: template.path + '_fork'
				}
			} catch (err) {
				script = defaultScript()
				console.error('Error loading template', err)
				sendUserToast('Error loading template: ' + err.message, true)
			}
		}
	}

	async function loadHub(): Promise<void> {
		if (hubPath) {
			try {
				const { content, language, summary } = await ScriptService.getHubScriptByPath({
					path: hubPath
				})
				script = {
					...defaultScript(),
					description: `Fork of ${hubPath}`,
					content: replaceScriptPlaceholderWithItsValues(hubPath, content),
					summary: summary ?? '',
					language: language as Script['language'],
					path: hubPath + '_fork'
				}
			} catch (err) {
				script = defaultScript()
				console.error('Error loading script from hub', err)
				sendUserToast('Error loading script from hub: ' + err.message, true)
			}
		}
	}

	loadHub()

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadTemplate())
		}
	})
</script>

{#if script}
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
		bind:script
		{showMeta}
		replaceStateFn={(path) => replaceState(path, $page.state)}
	>
		<UnsavedConfirmationModal
			getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
		/>
	</ScriptBuilder>
{:else}
	<ScriptEditorSkeleton />
{/if}
