<script lang="ts">
	import { ScriptService, type NewScript, type NewScriptWithDraft } from '$lib/gen'

	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft, type UserDraftHandle } from '$lib/userDraft.svelte'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// `initialArgs` is intentionally captured once at mount — it's the
	// session's initial argument set, not per-script.
	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	// Derived so client-side nav (breadcrumb) re-reads the URL, not mount-time values.
	let topHash = $derived(page.url.searchParams.get('topHash') ?? undefined)

	let hash = $derived(page.url.searchParams.get('hash') ?? undefined)

	// When viewing a specific historical hash we don't want a draft — that view
	// is read-only relative to drafts.
	let draftPath = $derived(hash ? '' : (page.params.path ?? ''))

	// `useMany` keyed off the reactive `draftPath` re-keys the handle on nav;
	// `scriptHandle` proxies the current handle so `bind:script` stays a fixed lvalue.
	const scriptHandles = UserDraft.useMany<EditableScript>(() => [
		{ itemKind: 'script', path: draftPath }
	])
	const scriptHandle: Pick<UserDraftHandle<EditableScript>, 'draft' | 'setInitial'> = {
		get draft() {
			return scriptHandles[0]?.draft
		},
		set draft(value) {
			const handle = scriptHandles[0]
			if (handle) handle.draft = value
		},
		setInitial(value) {
			scriptHandles[0]?.setInitial(value)
		}
	}

	$effect(() => {
		if (hash || !$workspaceStore) return
		const workspace = $workspaceStore
		UserDraft.setLiveEditorDraft({
			workspace,
			itemKind: 'script',
			storagePath: draftPath,
			effectivePath: scriptHandle.draft?.path ?? draftPath
		})
		return () => UserDraft.clearLiveEditorDraft('script', { workspace, storagePath: draftPath })
	})

	/** Some pages base64-JSON-encode a NewScript-like payload into the URL
	 * hash on `/scripts/edit/<path>#…` (e.g. Fork links). Treat it as a one-shot
	 * seed: apply it over the loaded baseline, then strip it from the URL. */
	function decodeUrlScriptSeed(): Partial<EditableScript> | undefined {
		const fragment = page.url.hash.startsWith('#') ? page.url.hash.slice(1) : ''
		if (!fragment) return undefined
		try {
			const decoded = JSON.parse(decodeURIComponent(atob(fragment)))
			if (decoded && typeof decoded === 'object') return decoded as Partial<EditableScript>
		} catch {
			// Hash isn't a valid encoded script — ignore.
		}
		return undefined
	}
	let urlScriptSeed = decodeUrlScriptSeed()

	// Seed from the URL so ScriptBuilder mounts with a populated `initialPath`.
	let initialPath: string = $state(hash ? '' : (page.params.path ?? ''))

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: NewScriptWithDraft | undefined = $state(undefined)
	let fullyLoaded = $state(false)

	// Remounts ScriptBuilder on nav: false while a reload runs, true once data is
	// ready. A synchronous `{#key}` swap instead races Monaco's init against the
	// torn-down container (mirrors how the raw-app editor clears `files`).
	let renderEditor = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	function applyBaseline(baseline: EditableScript): void {
		initialPath = baseline.path
		scriptBuilder?.setDraftTriggers(baseline.draft_triggers)
		scriptBuilder?.setCode(baseline.content)
		if (baseline['primary_schedule']) {
			savedPrimarySchedule = baseline['primary_schedule']
			scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
		}
	}

	/** Increments per `loadScript` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadScriptToken = 0
	async function loadScript(): Promise<void> {
		const tok = ++loadScriptToken
		fullyLoaded = false
		if (hash) {
			const scriptByHash = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash
			})
			if (tok !== loadScriptToken) return
			savedScript = structuredClone($state.snapshot(scriptByHash)) as NewScriptWithDraft
			scriptHandle.setInitial({ ...scriptByHash, parent_hash: hash, lock: undefined })
		} else {
			const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
				workspace: $workspaceStore!,
				path: page.params.path ?? ''
			})
			if (tok !== loadScriptToken) return
			savedScript = structuredClone($state.snapshot(scriptWithDraft))

			// The editor works off the backend DB draft when present, otherwise
			// the deployed version. Seeding (not assigning) avoids writing this
			// freshly-loaded value straight back to the DB.
			const backendDraft = scriptWithDraft.draft
				? ({ ...scriptWithDraft.draft } as EditableScript)
				: undefined
			const baseline = (backendDraft ?? (scriptWithDraft as EditableScript)) as EditableScript
			let bakedBaseline: EditableScript = {
				...baseline,
				parent_hash: topHash ?? scriptWithDraft.hash
			}
			if (urlScriptSeed) {
				bakedBaseline = { ...bakedBaseline, ...urlScriptSeed } as EditableScript
				urlScriptSeed = undefined
			}
			scriptHandle.setInitial(bakedBaseline)
		}

		if (scriptHandle.draft) {
			applyBaseline(scriptHandle.draft)
		}
		fullyLoaded = true
		renderEditor = true
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one script editor
		// to another (e.g. via the workspace picker) reloads the new script.
		page.params.path
		if ($workspaceStore) {
			untrack(() => {
				renderEditor = false // remount the builder for the navigated-to script
				loadScript().catch((e: any) => {
					// A failed load must NOT leave renderEditor stuck false — otherwise
					// the editor pane disappears and never remounts.
					console.error('Failed to load script', e)
					sendUserToast(`Failed to load script: ${e?.body ?? e?.message ?? e}`, true)
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDraft() {
		if (!savedScript || !savedScript.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Re-seed from the backend draft (drops the in-memory edits).
		goto(`/scripts/edit/${savedScript.draft.path}`)
		loadScript()
	}

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Delete the DB draft and reload the deployed version.
		UserDraft.discard('script', draftPath, undefined)
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
{#if scriptHandle.draft && renderEditor}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		bind:script={scriptHandle.draft}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={page.url.searchParams}
		onDeploy={(e) => {
			// "Deploy & Stay here" / lib: stay on the editor (just confirm).
			if (e.stay) {
				sendUserToast('Deployed')
				return
			}
			UserDraft.remove('script', draftPath)
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSaveInitial={(e) => {
			goto(`/scripts/edit/${e.path}`)
		}}
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onNavigate={(item) => goto(editPathFor(item))}
	>
		<UnsavedConfirmationModal
			{diffDrawer}
			getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
		/>
	</ScriptBuilder>
{/if}
