<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { initialArgsStore, userStore, workspaceStore } from '$lib/stores'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import { decodeState, emptySchema, type StateStore } from '$lib/utils'
	import { importFlowStore, initFlow } from '$lib/components/flows/flowStore.svelte'
	import { goto } from '$lib/navigation'

	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import { usePageDraftSync } from '$lib/components/usePageDraftSync.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { tick, untrack } from 'svelte'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { armRestartOnFirstInteraction } from '$lib/userDraftToast'

	let version: undefined | number = $state(undefined)

	// `initialArgs` is captured once at mount — it's the session's initial
	// argument set. The flow draft itself lives in UserDraft and is re-read
	// per loadFlow() so picker navigation doesn't reuse the original path's
	// state.
	const urlArgs = page.url.searchParams.get('initial_args')

	let initialArgs = $state({})
	if (urlArgs) {
		initialArgs = decodeState(urlArgs)
	} else if ($initialArgsStore) {
		initialArgs = $initialArgsStore
		$initialArgsStore = undefined
	}

	let savedFlow: Flow | undefined = $state(undefined)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)
	// `initialPath` is the editor-displayed path. Defaults to the URL
	// path so a deployed (or existing-draft) reload mounts FlowBuilder
	// with the real path; cleared to '' inside the `new_draft` branch
	// below so the Path widget's `initPath` calls `reset()` and seeds
	// the friendly `<random_adj>_flow` name on `/flows/add`.
	let flowInitialPath = $state(page.params.path ?? '')

	// Derived so client-side nav (breadcrumb) re-keys the handle to the new path.
	let flowDraftPath = $derived(page.params.path ?? '')

	/** True when no deployed flow exists at the URL path — either we
	 * landed via `/flows/add → ?new_draft=true` (no fetch yet) or the
	 * fetch came back with `no_deployed`. Controls FlowBuilder's deploy
	 * flow: `createFlow` vs `updateFlow`. Flips back to false once a
	 * deploy lands at this path. */
	let isNewFlow = $state(false)

	// Single page-level draft orchestration (handle re-keyed on nav,
	// recordRemoteSync, remove). `flowStore` reads/writes `draftSync.draft`.
	// `effectivePath` is omitted: the live-editor-draft registry entry for
	// flows is owned by FlowBuilder (via `liveEditorDraftStoragePath`).
	const draftSync = usePageDraftSync<Flow>({
		itemKind: 'flow',
		path: () => flowDraftPath,
		workspace: () => $workspaceStore
	})

	function emptyFlow(): Flow {
		return {
			summary: '',
			value: { modules: [] },
			path: '',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}
	}

	export const flowStore: StateStore<Flow> = {
		get val() {
			return draftSync.draft ?? emptyFlow()
		},
		set val(v: Flow) {
			draftSync.draft = v
		}
	}
	const flowStateStore = $state({ val: {} })

	let loading = $state(false)

	// Remounts FlowBuilder on nav: false while a reload runs, true once data is
	// ready, so it mounts fresh instead of reusing the previous flow's state.
	let renderEditor = $state(false)

	let selectedId: string = $state('settings-metadata')

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	let draftTriggersFromUrl: Trigger[] | undefined = $state(undefined)
	let selectedTriggerIndexFromUrl: number | undefined = $state(undefined)
	let loadedFromHistoryFromUrl:
		| { flowJobInitial: boolean | undefined; stepsState: Record<string, stepState> }
		| undefined = $state(undefined)

	let flowBuilder: FlowBuilder | undefined = $state(undefined)
	let notFound = $state(false)
	/** Increments per `loadFlow` call. Each in-flight load checks its captured
	 * token against this before writing shared state — if a newer load started
	 * (e.g. picker navigation while a draft-discard reload is in flight),
	 * the older promise no-ops at the next checkpoint. */
	let loadFlowToken = 0
	async function loadFlow(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadFlowToken
		loading = true
		let flow: Flow
		// Builder-dependent setup is captured here and applied AFTER the builder
		// remounts (see end of loadFlow): during a reload renderEditor is false,
		// so flowBuilder is unmounted and direct calls would no-op.
		let draftTriggersToApply: Trigger[] | undefined = undefined
		let applyPrimarySchedule = false
		// `?new_draft=true` (set by `/flows/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip both the latest-version and the get-by-path
		// fetches (they would 404), seed an empty Flow with `path = ''`
		// — the `Path` widget's `initPath` calls `reset()` only when
		// both `path` and `initialPath` are empty, and that's what
		// generates the friendly `<random_adj>_flow` name. Anything
		// non-empty (even `u/{user}/`) is parsed verbatim and the
		// friendly seed never fires.
		if (page.url.searchParams.get('new_draft') === 'true') {
			// No deployed row at the autogenerated `draft_{uuid}` URL
			// path — FlowBuilder's Deploy button must `createFlow` at
			// the user-typed path, not `updateFlow` at the URL.
			isNewFlow = true
			// The page component is reused across same-route navigation
			// (e.g. forking from an editor with collaborators) — clear the
			// previous path's draft-presence state so its hints and
			// stale-draft timestamps don't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// Suspend autosave around the bootstrap cascade: the Path
			// widget's `$workspaceStore && $userStore`-gated `initPath →
			// reset → onMetaChange → bind:path` chain seeds a friendly
			// auto-name into `$pathStore`, which FlowBuilder's
			// `draft_path` effect mirrors into `flow.draft_path`. That
			// programmatic write would otherwise fire as the user's
			// first save before they've touched anything. Resume on the
			// first real interaction (or fall back after 5s) so any
			// length of cascade is covered without picking a fragile
			// per-editor timer.
			if ($workspaceStore) {
				UserDraft.stopSync('flow', flowDraftPath, { workspace: $workspaceStore })
				armRestartOnFirstInteraction($workspaceStore, 'flow', flowDraftPath)
			}
			// Clear `initialPath` so the Path widget's `initPath` falls
			// into the empty-path branch and calls `reset()` to seed a
			// friendly auto-name (mirrors the script editor — without
			// this the topbar would just show the raw `draft_{uuid}`).
			flowInitialPath = ''
			// Capture every seeding intent BEFORE touching the URL — all of
			// these used to be consumed by /flows/add and are preserved
			// verbatim by its redirect.
			const hubId = page.url.searchParams.get('hub')
			const templatePath = page.url.searchParams.get('template')
			const templateId = page.url.searchParams.get('template_id')
			const isFork = page.url.searchParams.get('fork')
			// Fork-preview handoff (run page's "Fork" on a flow preview):
			// the state rides in localStorage, or on the opener window when
			// the flow was too large for localStorage. The URL hash carries
			// the same `{flow, path, initialArgs, draft_triggers,
			// selected_trigger, selectedId}` shape and wins when present.
			let forkState: any = undefined
			if (isFork) {
				const forkJson = localStorage.getItem('fork_flow')
				if (forkJson) {
					try {
						forkState = JSON.parse(forkJson)
					} catch {}
					localStorage.removeItem('fork_flow')
				} else if ((window.opener as any)?.__forkPreviewData) {
					forkState = (window.opener as any).__forkPreviewData
					delete (window.opener as any).__forkPreviewData
				}
			}
			if (page.url.hash != '') {
				forkState = decodeState(page.url.hash.slice(1))
			}
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			const empty: Flow = {
				path: '',
				summary: '',
				description: '',
				value: { modules: [] },
				schema: {},
				extra_perms: {},
				edited_at: new Date().toISOString(),
				edited_by: ''
			} as unknown as Flow
			// One-shot import handoff: "Import from YAML/JSON"
			// (CreateActionsFlow) writes $importFlowStore and routes to
			// /flows/add, which redirects here. Layer the imported flow
			// over the empty template; `path` stays '' so the Path widget
			// still generates the friendly name.
			const imported = $importFlowStore
			if (imported) {
				$importFlowStore = undefined
				sendUserToast('Flow loaded from YAML/JSON')
			}
			// Seed selection, in main's /flows/add priority order: YAML/JSON
			// import > fork/URL-state > template fork > hub fork > empty.
			// Fork seeds carry an explicit path suggestion (parsed verbatim
			// by the Path widget); the others keep `path = ''` for the
			// friendly auto-name.
			const forkOwner = `u/${$userStore?.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')}`
			let seed: Flow = empty
			let seedSelectedId: string | undefined = undefined
			if (imported) {
				seed = { ...empty, ...imported, path: '', extra_perms: {} } as unknown as Flow
			} else if (!templatePath && !hubId && forkState) {
				seed = {
					...empty,
					...forkState.flow,
					path: forkState.path ?? '',
					extra_perms: {}
				} as unknown as Flow
				if (forkState.initialArgs) {
					initialArgs = forkState.initialArgs
				}
				draftTriggersFromUrl = forkState.draft_triggers
				selectedTriggerIndexFromUrl = forkState.selected_trigger
				seedSelectedId = forkState.selectedId
			} else if (templatePath) {
				try {
					const template = templateId
						? await FlowService.getFlowVersion({
								workspace: $workspaceStore!,
								version: parseInt(templateId)
							})
						: await FlowService.getFlowByPath({
								workspace: $workspaceStore!,
								path: templatePath
							})
					if (tok !== loadFlowToken) return
					const oldPath = templatePath.split('/')
					seed = {
						...empty,
						...template,
						path: `${forkOwner}/${oldPath[oldPath.length - 1]}_fork`,
						extra_perms: {}
					} as unknown as Flow
				} catch (err: any) {
					if (tok !== loadFlowToken) return
					console.error('Error loading template', err)
					sendUserToast('Error loading template: ' + (err.body ?? err.message), true)
				}
			} else if (hubId) {
				try {
					const hub = await FlowService.getHubFlowById({ id: Number(hubId) })
					if (tok !== loadFlowToken) return
					delete (hub as any)['comments']
					seed = {
						...empty,
						...hub.flow,
						path: `${forkOwner}/flow_${hubId}`,
						extra_perms: {}
					} as unknown as Flow
					if (seed.value.preprocessor_module?.value.type === 'rawscript') {
						seed.value.preprocessor_module.value.content = replaceScriptPlaceholderWithItsValues(
							hubId,
							seed.value.preprocessor_module.value.content
						)
					}
					seedSelectedId = 'constants'
				} catch (err: any) {
					if (tok !== loadFlowToken) return
					console.error('Error loading hub flow', err)
					sendUserToast('Error loading hub flow: ' + (err.body ?? err.message), true)
				}
			}
			savedFlow = structuredClone(empty)
			draftSync.draft = seed
			flow = seed
			await initFlow(flow, flowStore, flowStateStore)
			if (tok !== loadFlowToken) return
			loading = false
			selectedId = page.url.searchParams.get('selected') ?? seedSelectedId ?? 'settings-metadata'
			renderEditor = true
			// Tutorial links ("/flows/add?tutorial=...") land here via the
			// redirect; fire once the builder has mounted and the flow input
			// anchor the tour points at exists.
			const tutorialParam = page.url.searchParams.get('tutorial')
			if (tutorialParam) {
				await tick()
				let attempts = 0
				while (attempts < 20 && !document.querySelector('#flow-editor-virtual-Input')) {
					await new Promise((resolve) => setTimeout(resolve, 100))
					attempts++
				}
				if (tok !== loadFlowToken) return
				flowBuilder?.triggerTutorial()
			}
			return
		}
		// Currently there is no way to get version of flow with flow.
		// So we have to request it here.
		//
		// Tolerate a missing version: when the path is draft-only (no
		// deployed flow at this path), `getFlowLatestVersion` returns no
		// row and `.id` would throw on `null`. The `getFlowByPath` call
		// below still returns the draft via the `get_draft` overlay, so
		// the editor can mount with `version = undefined`.
		let v: number | undefined
		try {
			v = (
				await FlowService.getFlowLatestVersion({
					workspace: $workspaceStore!,
					path: page.params.path ?? ''
				})
			)?.id
		} catch {
			v = undefined
		}
		if (tok !== loadFlowToken) return
		version = v

		const backendFlow = await FlowService.getFlowByPath({
			workspace: $workspaceStore!,
			path: page.params.path ?? '',
			getDraft
		})
		if (tok !== loadFlowToken) return
		otherDraftsUsers = (backendFlow.other_drafts_users ?? []) as OtherDraftUser[]
		draftSync.recordRemoteSync(backendFlow.draft_saved_at as string | undefined)
		// Re-evaluate the "new flow" signal on each load — flips to
		// true for draft-only paths and back to false once a deploy
		// lands at this URL path.
		isNewFlow = !!backendFlow.no_deployed
		if (backendFlow.is_draft) {
			loadedFromDraft = true
		}
		// Flow's `edited_at` is the deploy timestamp (sourced from
		// `flow_version.created_at`); compare against the draft's own
		// save time so DraftEditorModals can decide whether to open
		// the StaleDraftModal.
		draftSavedAt = backendFlow.draft_saved_at as string | undefined
		deployedAt = backendFlow.edited_at as string | undefined
		// `backendFlow` is the deployed payload; the user's saved draft
		// (if any) is attached as `.draft`. Layer the draft over the
		// deployed at the field level so downstream code sees the
		// "draft-on-deployed" shape it used to get pre-merged from the
		// server. See /scripts/edit's loader for the rationale.
		const { draft: draftFromBackend, ...deployedFlow } = backendFlow as any
		const effectiveFlow: Flow = draftFromBackend
			? ({ ...deployedFlow, ...draftFromBackend } as Flow)
			: (deployedFlow as Flow)
		savedFlow = structuredClone($state.snapshot(effectiveFlow)) as Flow
		// Reload of a draft-only (or rename-in-progress) flow: surface
		// the previously-typed `draft_path` to the Path widget so the
		// topbar shows the user's pending name instead of the
		// autogenerated `u/{user}/draft_{uuid}` URL slot. Without this
		// the widget seeds from `page.params.path`, the user's first
		// edit clobbers `draft.draft_path` back to the URL path, and
		// the home list silently loses the friendly name.
		const renderedDraftPath = (effectiveFlow as any).draft_path as string | undefined
		if (renderedDraftPath) flowInitialPath = renderedDraftPath

		// Backend canonical: overwrite the in-memory cell with the
		// effective (deployed+draft overlay) flow. The first cell write
		// after `acquireEntry` is swallowed by the syncer's seed guard,
		// so this load doesn't POST.
		flow = effectiveFlow
		draftSync.draft = effectiveFlow

		flowBuilder?.setDraftTriggers(undefined)

		await initFlow(flow, flowStore, flowStateStore)
		if (tok !== loadFlowToken) return
		loading = false
		selectedId = page.url.searchParams.get('selected') ?? 'settings-metadata'
		// Remount the builder first, then apply builder-dependent setup once it
		// has mounted — otherwise (during a reload) these would no-op on the
		// unmounted builder and editor state restoration would be skipped.
		renderEditor = true
		await tick()
		if (tok !== loadFlowToken) return
		if (applyPrimarySchedule) flowBuilder?.setPrimarySchedule(savedPrimarySchedule)
		flowBuilder?.setDraftTriggers(draftTriggersToApply)
		flowBuilder?.loadFlowState()
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one flow editor
		// to another (e.g. via the workspace picker) reloads the new flow.
		page.params.path
		if ($workspaceStore) {
			untrack(() => {
				renderEditor = false // remount the builder for the navigated-to flow
				loadFlow().catch((e: any) => {
					// A failed load must NOT leave renderEditor stuck false — otherwise
					// the editor pane disappears and never remounts. Surface the error
					// and remount so the user isn't stranded on a blank pane.
					console.error('Failed to load flow', e)
					sendUserToast(`Failed to load flow: ${e?.body ?? e?.message ?? e}`, true)
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDeployed() {
		if (!savedFlow) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		draftSync.remove()
		draftSync.draft = undefined
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} isFlow />
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="flow"
	path={flowDraftPath}
	{otherDraftsUsers}
	onLoadFromServer={() => loadFlow()}
	getLocalDraft={() => draftSync.draft}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	onLoadLatestDeploy={async () => {
		draftSync.draft = undefined
		await loadFlow({ getDraft: false })
	}}
/>
{#if notFound}
	<div class="flex flex-col items-center justify-center h-full">
		<h1 class="text-2xl font-bold">Flow not found at path {page.params.path}</h1>
		<p class="text-gray-500">The flow you are looking for does not exist.</p>
	</div>
{:else if renderEditor}
	<FlowBuilder
		onDeploy={(e) => {
			draftSync.remove()
			if ($workspaceStore) invalidate($workspaceStore, 'flow')
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onDetails={(e) => {
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onHistoryRestore={() => {
			loadFlow()
		}}
		onResetToDeployed={async () => {
			draftSync.draft = undefined
			await loadFlow({ getDraft: false })
		}}
		{loadedFromDraft}
		othersDraftsCount={otherDraftsUsers.length}
		onOpenOthersDrafts={() => (othersModalOpen = true)}
		onNavigate={(item) => goto(editPathFor(item))}
		{flowStore}
		{flowStateStore}
		bind:initialPath={flowInitialPath}
		liveEditorDraftStoragePath={flowDraftPath}
		newFlow={isNewFlow}
		{selectedId}
		{initialArgs}
		{loading}
		bind:this={flowBuilder}
		bind:savedFlow
		{diffDrawer}
		{savedPrimarySchedule}
		{draftTriggersFromUrl}
		{selectedTriggerIndexFromUrl}
		{version}
		{loadedFromHistoryFromUrl}
	/>
{/if}
