<script lang="ts">
	import { type NewScript, ScriptService, type ScriptLang } from '$lib/gen'

	import { page } from '$app/state'
	import { defaultScripts, initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import type { Schema } from '$lib/common'
	import {
		cleanValueProperties,
		decodeState,
		emptySchema,
		emptyString,
		encodeState,
		orderedJsonStringify,
		readFieldsRecursively,
		sendUserToast
	} from '$lib/utils'
	import { goto } from '$lib/navigation'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
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

	/** Some pages (run/[...run]'s "Fork" action, workspace_settings'
	 * error/success-handler template buttons) base64-JSON-encode a NewScript
	 * payload into the URL hash. That value is an explicit "open this script"
	 * intent and wins over local autosave, templates, hubs, and YAML imports.
	 *
	 * We can't use `decodeState` from utils.ts directly — it fires its own
	 * "Impossible to parse state" toast on failure, which would noise up the
	 * UI when the hash isn't a script payload at all (e.g. a route anchor).
	 */
	function decodeUrlScript(): Partial<Script> | undefined {
		const fragment = page.url.hash.startsWith('#') ? page.url.hash.slice(1) : ''
		if (!fragment) return undefined
		try {
			const decoded = JSON.parse(decodeURIComponent(atob(fragment)))
			if (decoded && typeof decoded === 'object') return decoded as Partial<Script>
		} catch {
			// Hash isn't a valid encoded script — ignore.
		}
		return undefined
	}
	const urlScript = decodeUrlScript()
	// "+ Script" buttons navigate with ?nodraft=true to signal "start fresh".
	// Wipe the persisted empty-path autosave and strip the flag from the URL
	// synchronously so a reload doesn't wipe the freshly-started draft. A
	// plain reload of /scripts/add (no nodraft) instead restores the
	// previous session.
	if (page.url.searchParams.get('nodraft') && typeof window !== 'undefined') {
		UserDraft.remove('script', '')
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}

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

	// templatePath/hubPath/import/url-hash flows replace the value before
	// render, so defaultValue is left undefined for those to avoid flashing a
	// blank editor.
	const scriptHandle = UserDraft.use<Script>('script', '', {
		defaultValue: templatePath || hubPath || urlScript ? undefined : defaultScript()
	})

	$effect(() => {
		if (!$workspaceStore) return
		const workspace = $workspaceStore
		UserDraft.setLiveEditorDraft({
			workspace,
			itemKind: 'script',
			storagePath: '',
			effectivePath: scriptHandle.draft?.path
		})
		return () => UserDraft.clearLiveEditorDraft('script', { workspace, storagePath: '' })
	})

	// === BEGIN TEMP URL-HASH SYNC (remove with future PR) ===
	// Legacy behavior: the URL hash both seeds the editor on load AND stays in
	// sync with edits (encoded back into the hash, debounced). Asks the user
	// via modal when the URL payload would clobber an existing local autosave.
	let urlConflictModalOpen = $state(false)
	let pendingUrlPayload: Script | undefined = undefined

	if (urlScript) {
		const seeded = { ...defaultScript(), ...urlScript } as Script
		const existing = scriptHandle.draft
		if (existing) {
			const localClean = orderedJsonStringify(cleanValueProperties(existing))
			const seededClean = orderedJsonStringify(cleanValueProperties(seeded))
			if (localClean !== seededClean) {
				pendingUrlPayload = seeded
				urlConflictModalOpen = true
			}
		} else {
			scriptHandle.draft = seeded
			sendUserToast('Loaded from URL')
		}
	}

	function onUrlConflictUseUrl() {
		if (pendingUrlPayload) {
			scriptHandle.draft = pendingUrlPayload
			sendUserToast('Loaded from URL')
		}
		pendingUrlPayload = undefined
		urlConflictModalOpen = false
	}
	function onUrlConflictKeepLocal() {
		pendingUrlPayload = undefined
		urlConflictModalOpen = false
	}

	let _urlHashSyncTimeout: number | undefined
	$effect(() => {
		const draft = scriptHandle.draft
		if (!draft) return
		// Gate while the conflict modal is open so we don't overwrite the URL
		// payload before the user has decided.
		if (urlConflictModalOpen) return
		readFieldsRecursively(draft)
		if (typeof window === 'undefined') return
		if (_urlHashSyncTimeout) clearTimeout(_urlHashSyncTimeout)
		_urlHashSyncTimeout = setTimeout(() => {
			const snapshot = $state.snapshot(scriptHandle.draft)
			if (!snapshot) return
			const url = new URL(window.location.href)
			url.hash = encodeState(snapshot)
			window.history.replaceState(window.history.state, '', url.toString())
		}, 500)
	})
	// === END TEMP URL-HASH SYNC ===

	async function loadTemplate(): Promise<void> {
		if (urlScript) return
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
		if (urlScript) return
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
	if (!urlScript && importParam && $importScriptStore) {
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

<!-- TEMP URL-HASH SYNC: conflict modal (remove with future PR) -->
<LocalDraftStaleModal
	open={urlConflictModalOpen}
	cause="url"
	onLoadLatest={onUrlConflictUseUrl}
	onKeepDraft={onUrlConflictKeepLocal}
/>

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
