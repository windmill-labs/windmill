<script lang="ts">
	import { run } from 'svelte/legacy'
	import { untrack } from 'svelte'

	import { AppService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { cleanValueProperties, orderedJsonStringify, readFieldsRecursively } from '$lib/utils'
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
	import { notifyDraftLoaded, notifyRestoredFromLocal } from '$lib/userDraftToast'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import OtherUsersDraftsModal from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'

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
				path: string
				summary: string
				policy: any
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''

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
	/** Mirrors `/apps/edit`'s `isNewApp`: true when no deployed row
	 * exists at the URL path. Flips RawAppEditor's deploy from
	 * `updateApp` to `createApp` so a user-typed path is used. */
	let isNewApp = $state(false)
	async function loadApp(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadAppToken
		// `?new_draft=true` (set by `/apps_raw/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip the backend fetch (it would 404), seed every
		// piece of state RawAppEditor needs (the template gates rendering
		// on `files`, so an unset files would leave the page blank —
		// which is the bug this branch fixes). Use `path = ''` so the
		// `Path` widget's `initPath` calls `reset()` and generates the
		// friendly `<random_adj>_raw_app` name. Strip the flag last.
		if (page.url.searchParams.get('new_draft') === 'true') {
			isNewApp = true
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			// Backend's `Policy` requires `execution_mode` — an empty
			// object fails to deserialize on deploy.
			const emptyPolicy = { execution_mode: 'publisher' } as any
			savedApp = {
				summary: '',
				value: { files: {}, runnables: {} },
				path: '',
				policy: emptyPolicy,
				custom_path: undefined
			}
			files = {}
			runnables = {}
			data = { ...DEFAULT_DATA }
			summary = ''
			policy = emptyPolicy
			newPath = ''
			return
		}
		const backendApp = (await AppService.getAppByPath({
			path: page.params.path ?? '',
			workspace: $workspaceStore!,
			getDraft,
			rawApp: true
		})) as any
		if (tok !== loadAppToken) return
		isNewApp = !!backendApp.no_deployed
		if (backendApp.is_draft) {
			notifyDraftLoaded({
				workspace: $workspaceStore!,
				itemKind: 'raw_app',
				path: page.params.path ?? '',
				draftOnly: backendApp.no_deployed,
				onResetToDeployed: async () => {
					draftHandle.setDraftAndMeta(undefined, {})
					await loadApp({ getDraft: false })
				}
			})
		}
		// Apply the user's saved draft. The autosave for raw apps writes a
		// flat `RawAppDraft` (`{files, runnables, data, summary, policy,
		// custom_path}`); the backend returns it in `.draft`. The rest of
		// this loader (and `extractRawApp` below) expects the deployed
		// shape where `files`/`runnables`/`data` live under `.value` and
		// the rest are top-level.
		//   - `no_deployed`: no deployed row exists. The response body is
		//     a best-effort stand-in equal to `.draft`. Synthesize the
		//     wrapper so downstream sees the editor's saved state under
		//     `.value` and as top-level metadata.
		//   - deployed + draft: keep the deployed metadata, replace the
		//     editable fields with the saved draft's.
		const savedRawAppDraft = backendApp.draft as
			| {
					files?: any
					runnables?: any
					data?: any
					summary?: string
					policy?: any
					custom_path?: string
			  }
			| undefined
		if (backendApp.no_deployed) {
			backendApp.value = {
				files: savedRawAppDraft?.files ?? {},
				runnables: savedRawAppDraft?.runnables ?? {},
				data: savedRawAppDraft?.data
			}
			backendApp.summary = savedRawAppDraft?.summary ?? ''
			// Backend's `Policy` requires `execution_mode` — fall back
			// to the publisher default when the saved draft (or
			// fetched payload) didn't carry one.
			backendApp.policy = savedRawAppDraft?.policy ?? { execution_mode: 'publisher' }
			backendApp.custom_path = savedRawAppDraft?.custom_path
			backendApp.path = page.params.path ?? ''
		} else if (savedRawAppDraft) {
			backendApp.value = {
				...(backendApp.value ?? {}),
				files: savedRawAppDraft.files ?? backendApp.value?.files ?? {},
				runnables: savedRawAppDraft.runnables ?? backendApp.value?.runnables ?? {},
				data: savedRawAppDraft.data ?? backendApp.value?.data
			}
			if (savedRawAppDraft.summary !== undefined) backendApp.summary = savedRawAppDraft.summary
			if (savedRawAppDraft.policy !== undefined) backendApp.policy = savedRawAppDraft.policy
			if (savedRawAppDraft.custom_path !== undefined)
				backendApp.custom_path = savedRawAppDraft.custom_path
		}
		const backendApp_ = structuredClone(stateSnapshot(backendApp))
		savedApp = {
			summary: backendApp_.summary,
			value: backendApp_.value as any,
			path: backendApp_.path,
			policy: backendApp_.policy,
			custom_path: backendApp_.custom_path
		}

		const backendSource: any = backendApp
		const localDraft = draftHandle.draft
		const previousMeta = draftHandle.meta
		const newRevs: UserDraftMeta = {
			remoteRev: backendApp.versions
				? backendApp.versions[backendApp.versions.length - 1]
				: undefined
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
			policy: backendSource.policy ?? backendApp.policy,
			custom_path: backendSource.custom_path ?? backendApp.custom_path
		}

		// Merge defaults from `backendBundle` first so a localDraft with a
		// missing key (e.g. legacy autosaves written before `policy` /
		// `custom_path` were added to the bundle) doesn't read as "user has
		// unsaved changes" and fire the restore toast on every open.
		const localBundle = localDraft != undefined ? { ...backendBundle, ...localDraft } : undefined
		if (
			localBundle != undefined &&
			orderedJsonStringify(cleanValueProperties(localBundle)) !==
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
				notifyRestoredFromLocal(false, true, {
					onResetToSavedDraft: () => {
						UserDraft.remove('raw_app', path)
						draftHandle.setDraftAndMeta(backendBundle, newRevs)
						extractRawApp(backendSource)
						redraw++
					},
					onResetToDeployed: async () => {
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
			runnables = localBundle.runnables
			data = localBundle.data
			summary = localBundle.summary
			policy = localBundle.policy ?? backendApp.policy
			newPath = backendApp.path
			files = localBundle.files
		} else {
			if (localDraft != undefined) UserDraft.remove('raw_app', path)
			extractRawApp(backendSource)
			draftHandle.setDraftAndMeta(backendBundle, newRevs)
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

	async function restoreDeployed() {
		if (!savedApp) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
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

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>
{#if $workspaceStore && path}
	<OtherUsersDraftsModal
		workspace={$workspaceStore}
		itemKind="raw_app"
		{path}
		currentValue={draftHandle.draft}
		currentUserEmail={$userStore?.email}
		{diffDrawer}
		userHasLocalDraft={UserDraft.has('raw_app', path)}
		onFork={(otherValue) => {
			UserDraft.save('raw_app', path, otherValue, { workspace: $workspaceStore })
			diffDrawer?.closeDrawer()
		}}
	/>
{/if}

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
				newApp={isNewApp}
			/>
		</div>
	{/key}
{/if}
