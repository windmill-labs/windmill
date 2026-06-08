<script lang="ts">
	import { ScriptService, type NewScript, type Script } from '$lib/gen'

	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { emptySchema } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { notifyDraftLoaded } from '$lib/userDraftToast'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// `initialArgs` is intentionally captured once at mount — it's the
	// session's initial argument set, not per-script.
	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	// Derived so client-side nav (breadcrumb) re-reads the URL, not mount-time values.
	let topHash = $derived(page.url.searchParams.get('topHash') ?? undefined)

	let hash = $derived(page.url.searchParams.get('hash') ?? undefined)

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	let draftPath = $derived(hash ? '' : (page.params.path ?? ''))

	// Re-keys the handle on nav (the reactive `draftPath` is read inside
	// the reconcile) while keeping `bind:script` a stable lvalue.
	const scriptHandle = UserDraft.useReactive<EditableScript>(() => ({
		itemKind: 'script',
		path: draftPath
	}))

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

	// Seed from the URL so ScriptBuilder mounts with a populated `initialPath`
	// even when `scriptHandle.draft` is already defined synchronously from a
	// local autosave. An empty initialPath flips ScriptBuilder's
	// `metadataOpen` heuristic (intended for /scripts/add) into "true" and
	// pops the settings drawer open on /edit.
	let initialPath: string = $state(hash ? '' : (page.params.path ?? ''))

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: Script | NewScript | undefined = $state(undefined)
	let fullyLoaded = $state(false)
	/** Other workspace users (and the legacy NULL-email row, if any) with
	 *  a draft on this path. Populated from the deployed-overlay response
	 *  on each `loadScript`; the banner opens when the list is non-empty. */
	let otherDraftsUsers = $state<OtherDraftUser[]>([])

	// Remounts ScriptBuilder on nav: false while a reload runs, true once data is
	// ready. A synchronous `{#key}` swap instead races Monaco's init against the
	// torn-down container (mirrors how the raw-app editor clears `files`).
	let renderEditor = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	/** Increments per `loadScript` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadScriptToken = 0
	async function loadScript(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadScriptToken
		fullyLoaded = false
		// `?new_draft=true` (set by `/scripts/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip the backend fetch (it would 404), seed an
		// empty `NewScript` whose `path` is intentionally empty: the
		// `Path` widget's `initPath` calls `reset()` when both `path`
		// and `initialPath` are empty, which is what generates the
		// friendly `<random_adj>_<kind>` name. Anything non-empty (even
		// `u/{user}/`) is parsed verbatim and the friendly seed never
		// fires. `initialPath = ''` also makes ScriptBuilder open the
		// metadata drawer on mount. Strip the single-use flag last.
		if (page.url.searchParams.get('new_draft') === 'true') {
			// Suspend autosave for the whole new-draft bootstrap: the seed
			// AND ScriptBuilder's `initContent` (which fills
			// `script.content` from a template) are both programmatic
			// writes that shouldn't appear on the server as the user's
			// first edit. ScriptBuilder lifts the suspension in
			// `initContent`'s `.finally`; this overlap is harmless (both
			// calls set the same flag).
			UserDraft.stopSync('script', draftPath)
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			const empty: EditableScript = {
				path: '',
				summary: '',
				description: '',
				content: '',
				language: 'bun',
				// MUST be `emptySchema()` (`{properties: {}, required: [],
				// type: 'object'}`), NOT `{}`. `inferArgs` does
				// `JSON.parse(JSON.stringify(schema.properties))` —
				// `JSON.stringify(undefined)` returns the value `undefined`,
				// and `JSON.parse(undefined)` coerces to the literal string
				// "undefined", throwing the toast "Could not parse code".
				schema: emptySchema(),
				// Mirrors the backend's overlay shape for deployed=null
				// paths — ScriptBuilder's Diff button gates on this so
				// /add (and any other draft-only state) doesn't offer a
				// diff that has no baseline to compare against.
				no_deployed: true
			} as unknown as EditableScript
			initialPath = ''
			savedScript = structuredClone(empty)
			scriptHandle.draft = empty
			fullyLoaded = true
			renderEditor = true
			return
		}
		if (hash) {
			const scriptByHash = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash
			})
			if (tok !== loadScriptToken) return
			savedScript = structuredClone($state.snapshot(scriptByHash))
			scriptHandle.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const backendScript = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: page.params.path ?? '',
				getDraft
			})
			if (tok !== loadScriptToken) return
			otherDraftsUsers = ((backendScript as any).other_drafts_users ?? []) as OtherDraftUser[]
			// Seed the per-tab `last_sync` map with the server's draft
			// timestamp so the next autosave attaches a matching
			// `last_sync` and the backend can reject stale writes.
			// `undefined` (no draft existed) clears the entry — the
			// next save then takes the "first push" branch on the server.
			if ($workspaceStore && page.params.path) {
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace: $workspaceStore, itemKind: 'script', path: page.params.path },
					(backendScript as any).draft_saved_at as string | undefined
				)
			}
			if (backendScript.is_draft) {
				notifyDraftLoaded({
					workspace: $workspaceStore!,
					itemKind: 'script',
					path: page.params.path ?? '',
					draftOnly: backendScript.no_deployed,
					onResetToDeployed: async () => {
						// Drop the in-memory draft and refetch *without* the
						// draft overlay — we don't trust the eventual delete
						// to have landed, so we read deployed directly.
						scriptHandle.draft = undefined
						await loadScript({ getDraft: false })
					}
				})
			}
			// `backendScript` is the deployed payload; the user's saved
			// draft (if any) sits in `.draft`. Layer the draft over the
			// deployed at the field level — the draft contributes editor
			// state (content, summary, …) and the deployed contributes
			// metadata the draft never carries (hash, version markers).
			const { draft: draftFromBackend, ...deployedScript } = backendScript as any
			const effectiveScript: EditableScript = draftFromBackend
				? { ...deployedScript, ...draftFromBackend }
				: (deployedScript as EditableScript)
			savedScript = structuredClone($state.snapshot(effectiveScript))
			// Backend is canonical: write the baked baseline into the
			// cell. `parent_hash` is grafted on so the editor's compile
			// reuses the deployed lock. The first cell write after
			// `acquireEntry` is swallowed by the syncer's seed guard, so
			// this load doesn't POST.
			scriptHandle.draft = {
				...effectiveScript,
				parent_hash: topHash ?? backendScript.hash
			}
		}

		if (scriptHandle.draft) {
			initialPath = scriptHandle.draft.path
			scriptBuilder?.setDraftTriggers(scriptHandle.draft.draft_triggers)
			scriptBuilder?.setCode(scriptHandle.draft.content)
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
					// the editor pane disappears and never remounts. Surface the error
					// and remount so the user isn't stranded on a blank pane.
					console.error('Failed to load script', e)
					sendUserToast(`Failed to load script: ${e?.body ?? e?.message ?? e}`, true)
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('script', draftPath)
		scriptHandle.draft = undefined
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<DraftEditorModals
	enabled={!hash}
	workspace={$workspaceStore ?? ''}
	itemKind="script"
	path={page.params.path ?? ''}
	{otherDraftsUsers}
	editPathFor={(forkedPath) => `/scripts/edit/${forkedPath}`}
	onLoadFromServer={() => loadScript()}
	getLocalDraft={() => scriptHandle.draft}
/>
{#if scriptHandle.draft && renderEditor}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		userDraftPath={draftPath}
		bind:script={scriptHandle.draft}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={page.url.searchParams}
		onResetToDeployed={async () => {
			scriptHandle.draft = undefined
			await loadScript({ getDraft: false })
		}}
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
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onNavigate={(item) => goto(editPathFor(item))}
	/>
{/if}
