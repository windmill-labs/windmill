<script lang="ts">
	import { run } from 'svelte/legacy'
	import { untrack } from 'svelte'

	import { AppService, DraftService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { readFieldsRecursively } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { page } from '$app/state'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import { UserDraft, localDraftDiffers } from '$lib/userDraft.svelte'

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
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''

	const draftHandle = UserDraft.use<RawAppDraft>('raw_app', path)

	// Persist the bundle whenever any of the pieces of state changes. The
	// handle's setter syncs the change to the DB (debounced); writes equal to
	// the seeded baseline are skipped, so loading doesn't write back.
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

	// Reflect an external UserDraft.save (e.g. from the AI copilot) into the
	// form. Idempotent; the `!files` guard skips the reload window.
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
		policy = app.policy
		newPath = app.path
	}

	/** Increments per `loadApp` call. Stale loads bail at the next checkpoint
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
			draft: app_w_draft_.draft,
			custom_path: app_w_draft_.custom_path
		}

		// The editor works off the backend DB draft when present, otherwise the
		// deployed version.
		const backendSource: any = app_w_draft.draft ? app_w_draft.draft : app_w_draft
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

		// Seed the handle (no DB write-back) before populating the form so the
		// persist effect's first write matches and is skipped.
		draftHandle.setInitial(backendBundle)
		extractRawApp(backendSource)
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
		// Explicit user action: delete the DB draft synchronously before reloading.
		if (savedApp.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'app',
				path: savedApp.path
			})
		}
		UserDraft.remove('raw_app', path)
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
