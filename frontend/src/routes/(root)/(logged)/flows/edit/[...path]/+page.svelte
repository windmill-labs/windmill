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
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { onDestroy, tick, untrack } from 'svelte'
	import { stripNewDraftFlagOnSave } from '$lib/newDraftFlag'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'
	import { UserDraft, draftValuesEqual } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import {
		armRestartOnFirstInteraction,
		discardDraftAfterDeploy,
		runResetToDeployed
	} from '$lib/userDraftToast'

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
	/** Deployed flow this load, the baseline the autosave `discardIf` compares
	 * against. `undefined` for draft-only paths (no deployed) so a draft-only
	 * item never self-destructs by "matching" a non-existent baseline. */
	let deployedBaseline = $state<Flow | undefined>(undefined)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)
	// The flow_version the draft was forked from (pinned, doesn't drift), for the
	// precise staleness check in DraftEditorModals + FlowBuilder's deploy guard.
	let draftBaseVersion = $state<number | undefined>(undefined)
	// Editor-displayed path; defaults to the URL path. Cleared to '' in the
	// `new_draft` branch so the Path widget's `initPath` seeds the friendly name.
	let flowInitialPath = $state(page.params.path ?? '')

	// Derived so client-side nav (breadcrumb) re-keys the handle to the new path.
	let flowDraftPath = $derived(page.params.path ?? '')

	/** No deployed flow at the URL path. Drives FlowBuilder's deploy:
	 * `createFlow` vs `updateFlow`. Flips false once a deploy lands here. */
	let isNewFlow = $state(false)

	// Page-level draft orchestration; `flowStore` reads/writes `draftSync.draft`.
	// `effectivePath` omitted: the live-editor-draft registry entry is owned by
	// FlowBuilder (via `liveEditorDraftStoragePath`).
	const draftSync = usePageDraftSync<Flow>({
		itemKind: 'flow',
		path: () => flowDraftPath,
		workspace: () => $workspaceStore,
		// Autosaves landing back on the deployed flow become deletes, so reverting
		// edits clears the draft instead of leaving a no-op behind.
		discardIf: (val) => deployedBaseline !== undefined && draftValuesEqual(val, deployedBaseline)
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
	/** Drops the previous load's `new_draft` strip-on-save listener. */
	let cleanupNewDraftFlag: (() => void) | undefined
	onDestroy(() => cleanupNewDraftFlag?.())
	async function loadFlow(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadFlowToken
		loading = true
		cleanupNewDraftFlag?.()
		cleanupNewDraftFlag = undefined
		let flow: Flow
		// Builder-dependent setup is captured here and applied AFTER the builder
		// remounts (see end of loadFlow): during a reload renderEditor is false,
		// so flowBuilder is unmounted and direct calls would no-op.
		let draftTriggersToApply: Trigger[] | undefined = undefined
		let applyPrimarySchedule = false
		// `?new_draft=true` (from `/flows/add`'s redirect): a fresh, never-saved
		// `draft_{uuid}` path. Skip both fetches (they 404) and seed empty with
		// `path = ''` for the friendly auto-name. See /scripts/edit's loader.
		if (page.url.searchParams.get('new_draft') === 'true') {
			// Deploy must `createFlow` at the user-typed path, not `updateFlow` at the URL.
			isNewFlow = true
			// Page reused across same-route nav: clear the previous path's
			// draft-presence state so it doesn't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// New-draft skips the deployed/draft fetches, so the version-staleness
			// inputs are never reassigned — clear the previous flow's values, else they
			// bleed across the reused route and falsely trip the stale-draft modal.
			version = undefined
			draftBaseVersion = undefined
			// Brand-new flow: no deployed baseline, so never discard-on-equal.
			deployedBaseline = undefined
			// Suspend autosave around the bootstrap cascade: the Path widget's
			// `initPath → reset → bind:path` chain seeds a friendly auto-name that
			// FlowBuilder mirrors into `flow.path` — a programmatic write that
			// must not post as the user's first save. Resume on first interaction
			// (with a 5s fallback) rather than guessing the cascade's length.
			if ($workspaceStore) {
				UserDraft.stopSync('flow', flowDraftPath, { workspace: $workspaceStore })
				armRestartOnFirstInteraction($workspaceStore, 'flow', flowDraftPath)
			}
			// Empty `initialPath` so `initPath` takes the `reset()` branch and seeds
			// the auto-name, else the topbar shows the raw `draft_{uuid}`.
			flowInitialPath = ''
			const hubId = page.url.searchParams.get('hub')
			const templatePath = page.url.searchParams.get('template')
			const templateId = page.url.searchParams.get('template_id')
			const isFork = page.url.searchParams.get('fork')
			// Explicit path seed: the fork-a-draft handoff re-homes the source
			// path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			// Fork-preview handoff (run page's "Fork"): state rides in localStorage,
			// or on the opener window when too large for it. The URL hash carries the
			// same shape and wins when present.
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
			// Keep `?new_draft=true` until the backend confirms the first autosave,
			// so a refresh before any edit re-seeds here instead of 404-ing on the
			// never-persisted `draft_{uuid}` path.
			if ($workspaceStore) {
				cleanupNewDraftFlag = stripNewDraftFlagOnSave({
					workspace: $workspaceStore,
					itemKind: 'flow',
					path: flowDraftPath
				})
			}
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
			// One-shot YAML/JSON import handoff via $importFlowStore; layered over
			// the empty template below with `path` '' for the friendly name.
			const imported = $importFlowStore
			if (imported) {
				$importFlowStore = undefined
				sendUserToast('Flow loaded from YAML/JSON')
			}
			// Seed priority: YAML/JSON import > fork/URL-state > template > hub >
			// empty. Fork seeds carry an explicit path; the others keep '' for the
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
			if (pathParam) {
				seed = { ...seed, path: pathParam } as Flow
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
		// Version isn't carried on the flow, so fetch it separately. Tolerate a
		// missing one: draft-only paths have no version row, but `getFlowByPath`
		// still returns the draft, so mount with `version = undefined`.
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
		// `other_drafts_users` only computed when `getDraft`; don't clobber the
		// known list on a `getDraft:false` reload. See /scripts/edit's loader.
		if (getDraft) {
			otherDraftsUsers = (backendFlow.other_drafts_users ?? []) as OtherDraftUser[]
		}
		draftSync.recordRemoteSync(backendFlow.draft_saved_at as string | undefined)
		// Re-evaluate per load: true for draft-only paths, false once deployed.
		isNewFlow = !!backendFlow.no_deployed
		// Per-response, NOT sticky: a later no-own-draft load in the same editor
		// must reset this so it can't wrongly force overlay mode.
		const hasOwnDraft = !!backendFlow.is_draft
		loadedFromDraft = hasOwnDraft
		// Pass both timestamps for DraftEditorModals' staleness compare: `edited_at`
		// is the deploy time (from `flow_version.created_at`), `draft_saved_at` the draft's.
		draftSavedAt = backendFlow.draft_saved_at as string | undefined
		deployedAt = backendFlow.edited_at as string | undefined
		// Layer the draft (`.draft`, if any) over the deployed payload at the field
		// level. See /scripts/edit's loader for the rationale.
		const { draft: draftFromBackend, ...deployedFlow } = backendFlow as any
		// `version_id` rides on the persisted draft (pinned at fork); undefined for a
		// pre-feature draft or when editing the deployed flow directly (no draft).
		draftBaseVersion = draftFromBackend?.version_id as number | undefined
		const effectiveFlow: Flow = draftFromBackend
			? ({ ...deployedFlow, ...draftFromBackend } as Flow)
			: (deployedFlow as Flow)
		savedFlow = structuredClone($state.snapshot(effectiveFlow)) as Flow
		// `savedFlow` is the deployed baseline (diff "deployed" side + restore
		// target), so its path must stay the deployed path even when the draft
		// overlay renamed `effectiveFlow.path`.
		if (!backendFlow.no_deployed) savedFlow.path = (deployedFlow as Flow).path
		// Baseline for the autosave `discardIf`: the deployed flow WITHOUT the
		// draft overlay (matches the unedited seed when no draft exists).
		deployedBaseline = backendFlow.no_deployed
			? undefined
			: (structuredClone($state.snapshot(deployedFlow)) as Flow)
		// `flowInitialPath` is the Path widget's rename baseline and the
		// `updateFlow` target on deploy. For a deployed flow it stays the URL
		// (deployed/original) path, while the draft's renamed path rides in
		// `effectiveFlow.path` (seeded into the Path widget via FlowBuilder's
		// `$pathStore`) — so the topbar shows the pending name without conflating
		// it with the original. A never-deployed draft (parked at a `draft_{uuid}`
		// storage key) has no original, so seed the baseline from its own friendly
		// path: else the storage key would be flagged as a rename target.
		if (backendFlow.no_deployed) {
			flowInitialPath = (effectiveFlow.path as string) || flowInitialPath
		}

		// "Load another user's draft" handoff: render their value over the deployed
		// metadata. Overlay mode (we have our own draft) never saves until the user
		// confirms overwriting it. See /scripts/edit's loader.
		const pendingLoad = getDraft
			? OtherUserDraftLoad.takePending($workspaceStore!, 'flow', flowDraftPath)
			: undefined
		// Revisiting a path whose overlay was never confirmed/reset: drop the stale
		// lock so editing our own draft works again. See /scripts/edit's loader.
		if (!pendingLoad && OtherUserDraftLoad.isActive($workspaceStore!, 'flow', flowDraftPath)) {
			OtherUserDraftLoad.clear($workspaceStore!, 'flow', flowDraftPath)
		}
		const flowToRender: Flow = pendingLoad
			? ({ ...deployedFlow, ...(pendingLoad.value as object) } as Flow)
			: effectiveFlow
		flow = flowToRender
		if (pendingLoad && hasOwnDraft) {
			OtherUserDraftLoad.beginOverlay({
				workspace: $workspaceStore!,
				itemKind: 'flow',
				path: flowDraftPath,
				ownerLabel: pendingLoad.ownerLabel,
				loadedValue: flowToRender,
				// Force a builder remount (like nav does) — FlowBuilder captures the
				// flow at mount, so reloading alone leaves the foreign graph on screen.
				onResetToOwnDraft: async () => {
					renderEditor = false
					await loadFlow({ getDraft: true })
				}
			})
			// Seed so the bound value updates WITHOUT a POST (the lock blocks it
			// anyway, but seeding avoids tripping the edit prompt).
			UserDraft.seed('flow', flowDraftPath, flowToRender, { workspace: $workspaceStore! })
		} else {
			// Overwrite the cell with the effective flow. The first cell write after
			// `acquireEntry` is swallowed by the syncer's seed guard, so no POST.
			draftSync.draft = flowToRender
		}

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
		if (!savedFlow || !$workspaceStore) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		goto(`/flows/edit/${savedFlow.path}`)
		// stopSync-bracketed delete + getDraft:false reload; a bare `remove()` +
		// `loadFlow()` loses the race. See /scripts/edit's restoreDeployed.
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'flow',
			path: flowDraftPath,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadFlow({ getDraft: false })
			}
		})
	}
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} isFlow />
<!-- Auto-save off: edits aren't persisted on leave, so warn before navigating
	away (and on tab close). Inert while auto-save is on. -->
<UnsavedConfirmationModal
	showAutosaveTips
	hasUnsavedChanges={() =>
		UserDraftDbSyncer.hasUnsavedDisabledChanges({
			workspace: $workspaceStore ?? '',
			itemKind: 'flow',
			path: flowDraftPath
		})}
	onDiscardChanges={() =>
		UserDraftDbSyncer.dropPending({
			workspace: $workspaceStore ?? '',
			itemKind: 'flow',
			path: flowDraftPath
		})}
/>
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="flow"
	path={flowDraftPath}
	{otherDraftsUsers}
	draftOnly={isNewFlow}
	hasOwnDraft={loadedFromDraft}
	onLoadFromServer={() => loadFlow()}
	getLocalDraft={() => draftSync.draft}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	{draftBaseVersion}
	deployedHeadVersion={version}
	onLoadLatestDeploy={async () => {
		// stopSync-bracketed; see /scripts/edit's restoreDeployed for the race.
		if (!$workspaceStore) return
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'flow',
			path: flowDraftPath,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadFlow({ getDraft: false })
			}
		})
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
			// stopSync-bracketed immediate delete; see /scripts/edit's restoreDeployed.
			if ($workspaceStore) {
				discardDraftAfterDeploy({
					workspace: $workspaceStore,
					itemKind: 'flow',
					path: flowDraftPath
				})
			}
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
		{draftBaseVersion}
		{loadedFromHistoryFromUrl}
	/>
{/if}
