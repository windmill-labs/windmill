<script lang="ts">
	import { run } from 'svelte/legacy'
	import { untrack } from 'svelte'

	import { AppService, DraftService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		readFieldsRecursively,
		type Value
	} from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { page } from '$app/state'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import {
		UserDraft,
		checkStaleness,
		localDraftDiffers,
		type UserDraftMeta
	} from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'

	type RawAppDraft = {
		files: Record<string, string>
		runnables: Record<string, any>
		data: RawAppData
		summary: string
		policy?: any
		custom_path?: string
	}

	let files: Record<string, string> | undefined = $state(undefined)
	let runnables = $state({})
	/** Data configuration including tables and creation policy */
	let data: RawAppData = $state({ ...DEFAULT_DATA })
	let newPath = $state('')
	// let lastVersion = 0
	let policy: any = $state({})
	let summary = $state('')

	let savedApp:
		| {
				value: {
					files: Record<string, { code: string }>
					runnables: Record<string, HiddenRunnable>
				}
				draft?: any
				path: string
				summary: string
				policy: any
				draft_only?: boolean
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''

	// `?nodraft=true` is the callers' way of saying "skip the local autosave
	// on this load." Wipe the UserDraft entry and strip the flag from the
	// URL synchronously, before the handle is created. A plain reload (no
	// nodraft) restores normally.
	if (page.url.searchParams.get('nodraft') && typeof window !== 'undefined') {
		UserDraft.remove('raw_app', path)
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}

	const draftHandle = UserDraft.use<RawAppDraft>('raw_app', path)

	// Local-draft staleness modal: opened when the remote has moved on since
	// the local autosave was written.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline:
		| { baseline: RawAppDraft; backendSource: any; revs: UserDraftMeta }
		| undefined = undefined

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		const { baseline, backendSource, revs } = pendingBaseline
		UserDraft.remove('raw_app', path)
		draftHandle.setDraftAndMeta(baseline, revs)
		extractRawApp(backendSource)
		pendingBaseline = undefined
		staleModalOpen = false
		redraw++
	}

	function onStaleKeepDraft(): void {
		if (pendingBaseline) {
			draftHandle.setMeta(pendingBaseline.revs, { force: true })
		}
		pendingBaseline = undefined
		staleModalOpen = false
	}

	// Persist the bundle whenever any of the four pieces of state changes.
	$effect(() => {
		const currentFiles = files
		if (!currentFiles) return
		readFieldsRecursively(currentFiles)
		readFieldsRecursively(runnables)
		readFieldsRecursively(data)
		readFieldsRecursively(policy)
		void summary
		draftHandle.draft = {
			files: currentFiles,
			runnables,
			data,
			summary,
			policy,
			custom_path: savedApp?.custom_path
		}
	})

	// Reflect an external UserDraft.save into the form. Idempotent; the
	// `!files` guard skips the reload window so it doesn't fight loadApp.
	$effect(() => {
		const d = draftHandle.draft
		const currentFiles = files
		if (d == null || !currentFiles) return
		untrack(() => {
			if (
				localDraftDiffers(d, {
					files: currentFiles,
					runnables,
					data,
					summary,
					policy,
					custom_path: savedApp?.custom_path
				})
			) {
				files = d.files
				runnables = d.runnables
				data = d.data
				summary = d.summary
				if (d.policy !== undefined) policy = d.policy
			}
		})
	})

	function extractRawApp(app: any) {
		runnables = app.value.runnables
		// Support old formats and new format
		if (app.value.data) {
			const d = app.value.data
			// Handle old nested creation format
			if (d.creation) {
				data = {
					tables: d.tables ?? [],
					datatable: d.creation.datatable,
					schema: d.creation.schema
				}
			} else {
				data = d
			}
		} else if (app.value.datatables) {
			data = { ...DEFAULT_DATA, tables: app.value.datatables }
		}
		files = app.value.files
		summary = app.summary
		// lastVersion = app.version
		policy = app.policy
		newPath = app.path
	}

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	async function loadApp(): Promise<void> {
		const tok = ++loadAppToken
		const app_w_draft = await AppService.getAppByPathWithDraft({
			path: page.params.path ?? '',
			workspace: $workspaceStore!
		})
		if (tok !== loadAppToken) return
		const app_w_draft_ = structuredClone(stateSnapshot(app_w_draft))
		savedApp = {
			summary: app_w_draft_.summary,
			value: app_w_draft_.value as any,
			path: app_w_draft_.path,
			policy: app_w_draft_.policy,
			draft_only: app_w_draft_.draft_only,
			draft: app_w_draft_.draft,
			custom_path: app_w_draft_.custom_path
		}

		const backendSource: any = app_w_draft.draft ? app_w_draft.draft : app_w_draft
		const localDraft = draftHandle.draft
		const previousMeta = draftHandle.meta
		const newRevs: UserDraftMeta = {
			remoteRev: app_w_draft.versions
				? app_w_draft.versions[app_w_draft.versions.length - 1]
				: undefined,
			remoteDraftRev: app_w_draft.draft_created_at
		}
		const backendBundle: RawAppDraft = {
			files: backendSource.value?.files ?? {},
			runnables: backendSource.value?.runnables ?? {},
			data:
				backendSource.value?.data ??
				(backendSource.value?.datatables
					? { ...DEFAULT_DATA, tables: backendSource.value.datatables }
					: { ...DEFAULT_DATA }),
			summary: backendSource.summary ?? '',
			policy: backendSource.policy ?? app_w_draft.policy,
			custom_path: backendSource.custom_path ?? app_w_draft.custom_path
		}

		if (
			localDraft != undefined &&
			orderedJsonStringify(cleanValueProperties(localDraft)) !==
				orderedJsonStringify(cleanValueProperties(backendBundle))
		) {
			const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
			if (cause) {
				pendingBaseline = { baseline: backendBundle, backendSource, revs: newRevs }
				staleModalCause = cause
				staleModalOpen = true
			} else {
				if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
					// Legacy entry — backfill meta so the next load can detect staleness.
					draftHandle.setMeta(newRevs, { force: true })
				}
				const appPath = app_w_draft.path
				const hasBackendDraft = app_w_draft.draft != undefined
				notifyRestoredFromLocal(hasBackendDraft, !app_w_draft.draft_only, {
					onResetToSavedDraft: () => {
						UserDraft.remove('raw_app', path)
						draftHandle.setDraftAndMeta(backendBundle, newRevs)
						extractRawApp(backendSource)
						redraw++
					},
					onResetToDeployed: async () => {
						if (hasBackendDraft) {
							await DraftService.deleteDraft({
								workspace: $workspaceStore!,
								kind: 'app',
								path: appPath
							})
						}
						UserDraft.remove('raw_app', path)
						// UserDraft.remove only clears localStorage. Drop the
						// entry's in-memory state too so loadApp doesn't re-read
						// the stale autosave and re-fire the same toast.
						draftHandle.setDraftAndMeta(undefined, {})
						await loadApp()
						redraw++
					}
				})
			}
			runnables = localDraft.runnables
			data = localDraft.data
			summary = localDraft.summary
			policy = localDraft.policy ?? app_w_draft.policy
			newPath = app_w_draft.path
			files = localDraft.files
		} else {
			if (localDraft != undefined) UserDraft.remove('raw_app', path)
			extractRawApp(backendSource)
			draftHandle.setDraftAndMeta(backendBundle, newRevs)

			if (app_w_draft.draft && !app_w_draft.draft_only) {
				const reloadAction = () => {
					extractRawApp(app_w_draft)
					redraw++
				}

				const deployed = cleanValueProperties(app_w_draft as Value)
				const draft = cleanValueProperties({ files, runnables })
				sendUserToast('app loaded from latest saved draft', false, [
					{
						label: 'Reset to deployed',
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
		}
	}

	run(() => {
		// Re-run on workspace OR path change so navigating from one raw app editor
		// to another (e.g. via the workspace picker) reloads the new app.
		const currentPath = page.params.path
		if ($workspaceStore && currentPath !== undefined) {
			// Clear files so RawAppEditor unmounts; it will remount when loadApp
			// completes with fresh data, re-initializing its internal stores.
			files = undefined
			path = currentPath
			loadApp()
		}
	})

	async function restoreDraft() {
		if (!savedApp || !savedApp.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('raw_app', path)
		// Drop the in-memory handle state so loadApp sees no local draft
		// on the next pass — otherwise the staleness check would compare
		// the stale in-memory meta against the freshly fetched backend and
		// fire a spurious modal.
		draftHandle.setDraftAndMeta(undefined, {})
		goto(`/apps/edit/${savedApp.draft.path}`)
		await loadApp()
		redraw++
	}

	async function restoreDeployed() {
		if (!savedApp) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		if (savedApp.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'app',
				path: savedApp.path
			})
		}
		UserDraft.remove('raw_app', path)
		draftHandle.setDraftAndMeta(undefined, {})
		goto(`/apps/edit/${savedApp.path}`)
		await loadApp()
		redraw++
	}

	let diffDrawer: DiffDrawer | undefined = $state(undefined)

	function onRestore(ev: any) {
		sendUserToast('App restored from previous deployment')
		let prev = ev.detail
		extractRawApp(prev)
		savedApp = {
			summary: prev.summary,
			value: structuredClone(stateSnapshot(prev.value)),
			path: prev.path,
			policy: structuredClone(stateSnapshot(policy)),
			custom_path: prev.custom_path
		}
		redraw++
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>

{#if files}
	{#key redraw}
		<div class="h-screen">
			<RawAppEditor
				on:savedNewAppPath={(event) => {
					UserDraft.remove('raw_app', path)
					goto(`/apps_raw/edit/${event.detail}`)
					newPath = event.detail
				}}
				on:restore={onRestore}
				bind:files
				bind:runnables
				bind:data
				bind:summary
				{newPath}
				path={page.params.path ?? ''}
				liveEditorDraftStoragePath={path}
				{policy}
				bind:savedApp
				{diffDrawer}
				newApp={false}
			/>
		</div>
	{/key}
{/if}
