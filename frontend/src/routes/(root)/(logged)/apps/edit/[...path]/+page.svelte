<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import OtherUsersDraftsModal from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { emptyApp } from '$lib/components/apps/editor/appUtils'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft, checkStaleness, type UserDraftMeta } from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'

	let app = $state(undefined as (AppWithLastVersion & { value: any }) | undefined)
	let savedApp:
		| {
				value: App
				path: string
				summary: string
				policy: any
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''

	// Local-draft staleness modal: opened when the remote has moved on since
	// the local autosave was written.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline:
		| { baseline: AppWithLastVersion & { value: any }; revs: UserDraftMeta }
		| undefined = undefined

	// Backend revs at the most recent `loadApp` — handed to AppEditor as
	// `initialRevs` so the very first local autosave persists with a meta
	// stamp. Without it the next reload's staleness check has nothing to
	// compare against and the first external deploy/draft slips through.
	let currentRevs = $state<UserDraftMeta | undefined>(undefined)

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		// `discard` (not `remove`) so the entry's in-memory state.val is
		// cleared synchronously. `redraw++` remounts AppEditor on the next
		// microtask, but Svelte may mount the new instance before the old
		// one's onDestroy releases its handle — the new instance would
		// then re-acquire the SAME entry whose state.val still has the
		// stale autosave, ignoring the just-emptied LS. Same reason every
		// "reset" path below uses discard.
		UserDraft.discard('app', path, undefined)
		currentRevs = pendingBaseline.revs
		app = pendingBaseline.baseline
		pendingBaseline = undefined
		staleModalOpen = false
		redraw++
	}

	function onStaleKeepDraft(): void {
		if (pendingBaseline) {
			UserDraft.saveMeta('app', path, pendingBaseline.revs)
		}
		pendingBaseline = undefined
		staleModalOpen = false
	}

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	async function loadApp(): Promise<void> {
		const tok = ++loadAppToken
		// `?new_draft=true` (set by `/apps/add`'s redirect) means we landed
		// on a fresh `u/{user}/draft_{uuid}` path that's never been saved.
		// Skip the backend fetch (it would 404), seed an empty app with
		// `path = ''` so the `Path` widget's `initPath` calls `reset()`
		// and generates the friendly `<random_adj>_app` name. Anything
		// non-empty (even `u/{user}/`) is parsed verbatim and the
		// friendly seed never fires. Strip the flag last.
		if (page.url.searchParams.get('new_draft') === 'true') {
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			const emptyValue = emptyApp()
			app = {
				summary: '',
				value: emptyValue as any,
				path: '',
				policy: {} as any,
				custom_path: undefined,
				versions: [] as any,
				id: 0 as any,
				extra_perms: {},
				created_at: new Date().toISOString(),
				created_by: '',
				raw_app: false
			} as unknown as AppWithLastVersion & { value: any }
			savedApp = {
				summary: '',
				value: emptyValue as any,
				path: '',
				policy: {} as any
			}
			currentRevs = {}
			return
		}
		const backendApp = await AppService.getAppByPath({
			path: page.params.path ?? '',
			workspace: $workspaceStore!,
			getDraft: true
		})
		if (tok !== loadAppToken) return
		if (backendApp.is_draft) {
			sendUserToast('Loaded your saved draft')
		}
		const backendApp_ = structuredClone(stateSnapshot(backendApp))
		savedApp = {
			summary: backendApp_.summary,
			value: backendApp_.value as App,
			path: backendApp_.path,
			policy: backendApp_.policy,
			custom_path: backendApp_.custom_path
		}

		const localDraftValue = UserDraft.get<App>('app', path)
		const previousMeta = UserDraft.getMeta('app', path)
		const newRevs: UserDraftMeta = {
			remoteRev: backendApp.versions
				? backendApp.versions[backendApp.versions.length - 1]
				: undefined
		}
		currentRevs = newRevs
		if (
			localDraftValue != undefined &&
			orderedJsonStringify(cleanValueProperties(localDraftValue)) !==
				orderedJsonStringify(cleanValueProperties(backendApp.value as any))
		) {
			const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
			if (cause) {
				pendingBaseline = { baseline: backendApp, revs: newRevs }
				staleModalCause = cause
				staleModalOpen = true
			} else {
				if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
					// Legacy entry — backfill meta so the next load can detect staleness.
					UserDraft.saveMeta('app', path, newRevs)
				}
				notifyRestoredFromLocal(false, true, {
					onResetToSavedDraft: () => {
						UserDraft.discard('app', path, undefined)
						currentRevs = newRevs
						app = backendApp
						redraw++
					},
					onResetToDeployed: async () => {
						UserDraft.discard('app', path, undefined)
						goto(`/apps/edit/${backendApp.path}`)
						await loadApp()
						redraw++
					}
				})
			}
			app = { ...backendApp, value: localDraftValue }
		} else {
			// Local is missing or matches backend — wipe any stale entry so it
			// doesn't haunt the next session and use the backend value.
			if (localDraftValue != undefined) UserDraft.remove('app', path)
			app = backendApp
		}
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one app editor
		// to another (e.g. via the workspace picker) reloads the new app.
		const currentPath = page.params.path
		if ($workspaceStore && currentPath !== undefined) {
			untrack(() => {
				// Clear the app so AppEditor unmounts; it will remount once loadApp
				// completes with fresh data, re-initializing its internal stores.
				app = undefined
				path = currentPath
				loadApp()
			})
		}
	})

	async function restoreDeployed() {
		if (!savedApp) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.discard('app', path, undefined)
		goto(`/apps/edit/${savedApp.path}`)
		await loadApp()
		redraw++
	}

	let diffDrawer: DiffDrawer | undefined = $state()

	function onRestore(ev: any) {
		sendUserToast('App restored from previous deployment')
		app = ev.detail
		const app_ = structuredClone(stateSnapshot(app!))
		savedApp = {
			summary: app_.summary,
			value: app_.value as App,
			path: app_.path,
			policy: app_.policy,
			custom_path: app_.custom_path
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
		itemKind="app"
		{path}
		currentValue={app?.value}
		currentUserEmail={$userStore?.email}
		{diffDrawer}
		userHasLocalDraft={UserDraft.has('app', path)}
		onFork={(otherValue) => {
			UserDraft.save('app', path, otherValue, { workspace: $workspaceStore })
			diffDrawer?.closeDrawer()
		}}
	/>
{/if}

{#key redraw}
	{#if app}
		<div class="h-screen">
			<AppEditor
				onSavedNewAppPath={(url) => {
					goto(`/apps/edit/${url}`)
					if (app) {
						app.path = url
					}
				}}
				on:restore={onRestore}
				summary={app.summary}
				app={app.value}
				newPath={app.path}
				path={page.params.path ?? ''}
				policy={app.policy}
				bind:savedApp
				{diffDrawer}
				version={app.versions ? app.versions[app.versions.length - 1] : undefined}
				newApp={false}
				initialRevs={currentRevs}
				replaceStateFn={(path) => replaceState(path, page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
			/>
		</div>
	{/if}
{/key}
