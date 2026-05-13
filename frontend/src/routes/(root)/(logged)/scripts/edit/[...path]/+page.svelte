<script lang="ts">
	import { ScriptService, type NewScript, type NewScriptWithDraft, DraftService } from '$lib/gen'

	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft, checkStaleness, type UserDraftMeta } from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// `initialArgs` is intentionally captured once at mount — it's the
	// session's initial argument set, not per-script.
	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	let topHash = page.url.searchParams.get('topHash') ?? undefined

	let hash = page.url.searchParams.get('hash') ?? undefined

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	const draftPath = hash ? '' : (page.params.path ?? '')
	const scriptHandle = UserDraft.use<EditableScript>('script', draftPath)

	/** Some pages base64-JSON-encode a NewScript-like payload into the URL
	 * hash on `/scripts/edit/<path>#…`. Treat it as a one-shot seed that
	 * wins over local autosave + backend draft + deployed: apply, toast,
	 * strip from the URL. Same logic as /scripts/add, kept in this file for
	 * a faithful mirror of its decoder.
	 *
	 * Can't reuse `decodeState` from utils.ts — it fires its own error toast
	 * on parse failure, which would noise up the UI for unrelated anchors.
	 */
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

	let initialPath: string = $state('')

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: NewScriptWithDraft | undefined = $state(undefined)
	let fullyLoaded = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	// Local-draft staleness modal: opened when the remote (deployed or DB
	// draft) has moved on since the user's autosave was created.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline: { baseline: EditableScript; revs: UserDraftMeta } | undefined = undefined

	function applyBaseline(baseline: EditableScript): void {
		initialPath = baseline.path
		scriptBuilder?.setDraftTriggers(baseline.draft_triggers)
		scriptBuilder?.setCode(baseline.content)
		if (baseline['primary_schedule']) {
			savedPrimarySchedule = baseline['primary_schedule']
			scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
		}
	}

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		const { baseline, revs } = pendingBaseline
		UserDraft.remove('script', draftPath)
		scriptHandle.setDraftAndMeta(baseline, revs)
		applyBaseline(baseline)
		pendingBaseline = undefined
		staleModalOpen = false
	}

	function onStaleKeepDraft(): void {
		if (pendingBaseline) {
			scriptHandle.setMeta(pendingBaseline.revs, { force: true })
		}
		pendingBaseline = undefined
		staleModalOpen = false
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
			scriptHandle.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
				workspace: $workspaceStore!,
				path: page.params.path ?? ''
			})
			if (tok !== loadScriptToken) return
			savedScript = structuredClone($state.snapshot(scriptWithDraft))

			const localDraft = scriptHandle.draft
			const previousMeta = scriptHandle.meta
			const backendDraft = scriptWithDraft.draft
				? ({ ...scriptWithDraft.draft } as EditableScript)
				: undefined
			const newRevs: UserDraftMeta = {
				remoteRev: scriptWithDraft.hash,
				remoteDraftRev: scriptWithDraft.draft_created_at
			}

			// Compute the fully-baked initial value once so the assignment
			// below is a single write — otherwise post-load mutations like
			// `parent_hash = ...` would count as a second write under
			// useLocalStorageValue's saveInitialValue=false contract and get
			// persisted before the user has touched anything.
			const baseline = (backendDraft ?? (scriptWithDraft as EditableScript)) as EditableScript
			const bakedBaseline: EditableScript = {
				...baseline,
				parent_hash: topHash ?? scriptWithDraft.hash
			}

			if (urlScriptSeed) {
				// URL hash seed wins over local autosave + backend draft + deployed.
				// Wipe the stale autosave on disk so the in-memory seed becomes the
				// only source — the user's subsequent edits will overwrite cleanly.
				const seeded = { ...bakedBaseline, ...urlScriptSeed } as EditableScript
				UserDraft.remove('script', draftPath)
				scriptHandle.setDraftAndMeta(seeded, newRevs)
				sendUserToast('Loaded from URL')
				// Consume-once: strip the hash so a reload restores the user's
				// autosave (which now reflects the seed + edits) instead of
				// re-applying the seed.
				if (typeof window !== 'undefined') {
					const url = new URL(window.location.href)
					url.hash = ''
					window.history.replaceState(window.history.state, '', url.toString())
				}
				urlScriptSeed = undefined
			} else if (localDraft != undefined) {
				const reference = backendDraft ?? scriptWithDraft
				const referenceClean = cleanValueProperties(reference)
				const localClean = cleanValueProperties(localDraft)
				if (orderedJsonStringify(referenceClean) === orderedJsonStringify(localClean)) {
					// Local matches the saved version — silently drop it and use the saved one.
					UserDraft.remove('script', draftPath)
					scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
				} else {
					const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
					if (cause) {
						// Remote moved on since the local autosave was written —
						// surface the choice via modal. The local draft stays on
						// screen until the user picks.
						pendingBaseline = { baseline: bakedBaseline, revs: newRevs }
						staleModalCause = cause
						staleModalOpen = true
					} else {
						if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
							// Legacy entry (no meta recorded) — backfill so future
							// loads can detect staleness even if the user doesn't edit.
							scriptHandle.setMeta(newRevs, { force: true })
						}
						const scriptPath = bakedBaseline.path
						const hasBackendDraft = !!backendDraft
						notifyRestoredFromLocal(hasBackendDraft, !scriptWithDraft.draft_only, {
							onResetToSavedDraft: () => {
								UserDraft.remove('script', draftPath)
								scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
								applyBaseline(bakedBaseline)
							},
							onResetToDeployed: async () => {
								if (hasBackendDraft) {
									await DraftService.deleteDraft({
										workspace: $workspaceStore!,
										kind: 'script',
										path: scriptPath
									})
								}
								UserDraft.remove('script', draftPath)
								// UserDraft.remove only clears localStorage. The entry's
								// in-memory state is kept alive by this route's handle, so
								// loadScript would re-read the stale autosave and the toast
								// would fire again. Drop the in-memory state first.
								scriptHandle.setDraftAndMeta(undefined, {})
								goto(`/scripts/edit/${scriptPath}`)
								loadScript()
							}
						})
					}
				}
			} else if (backendDraft) {
				scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
				if (bakedBaseline['primary_schedule']) {
					savedPrimarySchedule = bakedBaseline['primary_schedule']
					scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
				}
				scriptBuilder?.setDraftTriggers(bakedBaseline.draft_triggers)

				if (!scriptWithDraft.draft_only) {
					const reloadAction = async () => {
						await DraftService.deleteDraft({
							workspace: $workspaceStore!,
							kind: 'script',
							path: bakedBaseline.path
						})
						UserDraft.remove('script', draftPath)
						goto(`/scripts/edit/${bakedBaseline.path}`)
						loadScript()
					}
					const deployed = cleanValueProperties(scriptWithDraft)
					const draft = cleanValueProperties(bakedBaseline)
					sendUserToast('Script loaded from latest saved draft', false, [
						{
							label: 'Discard draft reset to deployed version',
							callback: reloadAction
						},
						{
							label: 'Show diff',
							callback: async () => {
								diffDrawer?.openDrawer()
								diffDrawer?.setDiff({
									mode: 'simple',
									original: deployed,
									current: draft,
									title: 'Deployed <> Draft',
									button: { text: 'Discard draft', onClick: reloadAction }
								})
							}
						}
					])
				}
			} else {
				scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
			}
		}

		if (scriptHandle.draft) {
			initialPath = scriptHandle.draft.path
			scriptBuilder?.setDraftTriggers(scriptHandle.draft.draft_triggers)
			scriptBuilder?.setCode(scriptHandle.draft.content)
		}
		fullyLoaded = true
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one script editor
		// to another (e.g. via the workspace picker) reloads the new script.
		page.params.path
		if ($workspaceStore) {
			untrack(() => loadScript())
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDraft() {
		if (!savedScript || !savedScript.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('script', draftPath)
		goto(`/scripts/edit/${savedScript.draft.path}`)
		loadScript()
	}

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		if (savedScript.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'script',
				path: savedScript.path
			})
		}
		UserDraft.remove('script', draftPath)
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>
{#if scriptHandle.draft}
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
			UserDraft.remove('script', draftPath)
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSaveInitial={(e) => {
			if ($workspaceStore) invalidate($workspaceStore, 'script')
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
