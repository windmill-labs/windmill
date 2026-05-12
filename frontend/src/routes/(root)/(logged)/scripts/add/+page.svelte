<script lang="ts">
	import { type NewScript, ScriptService, type ScriptLang } from '$lib/gen'

	import { page } from '$app/state'
	import { defaultScripts, initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import type { Schema } from '$lib/common'
	import { decodeState, emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import ScriptEditorSkeleton from '$lib/components/ScriptEditorSkeleton.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
	import { isWorkflowAsCode } from '$lib/components/graph/wacToFlow'
	import { UserDraft } from '$lib/userDraft.svelte'

	type Script = NewScript & {
		draft_triggers?: Trigger[]
		hash?: string
		extra_perms?: Record<string, any>
	}

	// Default
	let schema: Schema = emptySchema()

	const templatePath = page.url.searchParams.get('template')
	const hubPath = page.url.searchParams.get('hub')
	const showMeta = /true|1/i.test(page.url.searchParams.get('show_meta') ?? '0')
	const urlArgs = page.url.searchParams.get('initial_args')
	const collabLang = page.url.searchParams.get('lang') as ScriptLang | null
	const wacParam = page.url.searchParams.get('wac')
	const importParam = page.url.searchParams.get('import')

	let initialArgs = urlArgs ? decodeState(urlArgs) : (get(initialArgsStore) ?? {})
	if (get(initialArgsStore)) $initialArgsStore = undefined

	const path = page.url.searchParams.get('path')

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

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
			language:
				(wacParam === 'python' ? 'python3' : wacParam === 'typescript' ? 'bun' : null) ??
				collabLang ??
				(($defaultScripts?.order?.filter(
					(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x)
				)?.[0] ?? 'bun') as ScriptLang),
			kind: 'script'
		}
	}

	// New script: keyed on '' (in-memory only — empty paths bypass localStorage).
	// templatePath/hubPath/import flows replace the value before render, so
	// defaultValue is left undefined for those to avoid flashing a blank editor.
	const scriptHandle = UserDraft.use<Script>('script', '', {
		defaultValue: templatePath || hubPath ? undefined : defaultScript()
	})

	async function loadTemplate(): Promise<void> {
		if (templatePath) {
			try {
				const template = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: templatePath
				})
				scriptHandle.draft = {
					...defaultScript(),
					summary: !emptyString(template.summary) ? `Copy of ${template.summary}` : '',
					description: template.description,
					content: template.content,
					schema: template.schema,
					language: template.language,
					path: template.path + '_fork'
				}
			} catch (err) {
				scriptHandle.draft = defaultScript()
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
				scriptHandle.draft = {
					...defaultScript(),
					description: `Fork of ${hubPath}`,
					content: replaceScriptPlaceholderWithItsValues(hubPath, content),
					summary: summary ?? '',
					language: language as Script['language'],
					path: hubPath + '_fork'
				}
			} catch (err) {
				scriptHandle.draft = defaultScript()
				console.error('Error loading script from hub', err)
				sendUserToast('Error loading script from hub: ' + err.message, true)
			}
		}
	}

	loadHub()

	let importedWacTemplate: 'wac_python' | 'wac_typescript' | undefined = undefined
	if (importParam && $importScriptStore) {
		const imported = $importScriptStore
		$importScriptStore = undefined
		const isWac = isWorkflowAsCode(imported.content ?? '', imported.language ?? '')
		scriptHandle.draft = {
			...defaultScript(),
			...imported,
			path: path ?? '',
			hash: '',
			extra_perms: {}
		}
		if (isWac) {
			importedWacTemplate = imported.language === 'python3' ? 'wac_python' : 'wac_typescript'
			sendUserToast('WAC script loaded from YAML/JSON')
		} else {
			sendUserToast('Script loaded from YAML/JSON')
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadTemplate())
		}
	})
</script>

{#if scriptHandle.draft}
	<ScriptBuilder
		{initialArgs}
		bind:this={scriptBuilder}
		lockedLanguage={templatePath != null || hubPath != null}
		template={importedWacTemplate ??
			(wacParam === 'python'
				? 'wac_python'
				: wacParam === 'typescript'
					? 'wac_typescript'
					: 'script')}
		onDeploy={(e) => {
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSaveInitial={(e) => {
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/edit/${e.path}`)
		}}
		onNavigate={(item) => goto(editPathFor(item))}
		searchParams={page.url.searchParams}
		bind:script={scriptHandle.draft}
		{showMeta}
	>
		<UnsavedConfirmationModal
			getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
		/>
	</ScriptBuilder>
{:else}
	<ScriptEditorSkeleton />
{/if}
