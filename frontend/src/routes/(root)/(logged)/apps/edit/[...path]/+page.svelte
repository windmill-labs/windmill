<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import {
		AppService,
		type AppWithLastVersion,
		type AppWithLastVersionWDraft,
		DraftService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { cleanValueProperties, orderedJsonStringify, type Value } from '$lib/utils'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft, checkStaleness, type UserDraftMeta } from '$lib/userDraft.svelte'

	let app = $state(
		undefined as (AppWithLastVersion & { draft_only?: boolean; value: any }) | undefined
	)
	let savedApp:
		| {
				value: App
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

	// Local-draft staleness modal: opened when the remote has moved on since
	// the local autosave was written.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline:
		| { baseline: AppWithLastVersion & { draft_only?: boolean; value: any }; revs: UserDraftMeta }
		| undefined = undefined

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		UserDraft.remove('app', path)
		UserDraft.saveMeta('app', path, pendingBaseline.revs)
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

	let nodraft = page.url.searchParams.get('nodraft')

	afterNavigate(() => {
		if (nodraft) {
			let url = new URL(page.url.href)
			url.search = ''
			replaceState(url.toString(), page.state)
		}
	})

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
		const app_w_draft_: AppWithLastVersionWDraft = structuredClone(stateSnapshot(app_w_draft))
		savedApp = {
			summary: app_w_draft_.summary,
			value: app_w_draft_.value as App,
			path: app_w_draft_.path,
			policy: app_w_draft_.policy,
			draft_only: app_w_draft_.draft_only,
			draft:
				app_w_draft_.draft?.['summary'] !== undefined // backward compatibility for old drafts missing metadata
					? app_w_draft_.draft
					: app_w_draft_.draft
						? {
								summary: app_w_draft_.summary,
								value: app_w_draft_.draft,
								path: app_w_draft_.path,
								policy: app_w_draft_.policy,
								custom_path: app_w_draft_.custom_path
							}
						: undefined,
			custom_path: app_w_draft_.custom_path
		}

		// Resolve the app value: backend draft > deployed, then overlay any
		// local autosave from UserDraft if present.
		const backendApp = app_w_draft.draft
			? app_w_draft.summary !== undefined
				? ({ ...app_w_draft, ...app_w_draft.draft } as AppWithLastVersion & {
						draft_only?: boolean
						value: any
					})
				: ({ ...app_w_draft, value: app_w_draft.draft } as AppWithLastVersion & {
						draft_only?: boolean
						value: any
					})
			: app_w_draft

		const localDraftValue = UserDraft.get<App>('app', path)
		const previousMeta = UserDraft.getMeta('app', path)
		const newRevs: UserDraftMeta = {
			remoteRev: app_w_draft.versions
				? app_w_draft.versions[app_w_draft.versions.length - 1]
				: undefined,
			remoteDraftRev: app_w_draft.draft_created_at
		}
		if (
			localDraftValue != undefined &&
			orderedJsonStringify(cleanValueProperties(localDraftValue)) !==
				orderedJsonStringify(cleanValueProperties(backendApp.value))
		) {
			const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
			if (cause) {
				pendingBaseline = { baseline: backendApp, revs: newRevs }
				staleModalCause = cause
				staleModalOpen = true
			} else if (
				previousMeta.remoteRev === undefined &&
				previousMeta.remoteDraftRev === undefined
			) {
				// Legacy entry — backfill meta so the next load can detect staleness.
				UserDraft.saveMeta('app', path, newRevs)
			}
			app = { ...backendApp, value: localDraftValue }
		} else {
			// Local is missing or matches backend — wipe any stale entry so it
			// doesn't haunt the next session and use the backend value.
			if (localDraftValue != undefined) UserDraft.remove('app', path)
			app = backendApp
		}

		if (app_w_draft.draft && !app_w_draft.draft_only && localDraftValue == undefined) {
			const reloadAction = () => {
				app = app_w_draft
				redraw++
			}

			const deployed = cleanValueProperties(app_w_draft as Value)
			const draft = cleanValueProperties(app ?? {})
			sendUserToast('app loaded from latest saved draft', false, [
				{
					label: 'Discard draft and load from latest deployed version',
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

	async function restoreDraft() {
		if (!savedApp || !savedApp.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('app', path)
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
		UserDraft.remove('app', path)
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

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>

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
				replaceStateFn={(path) => replaceState(path, page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
			>
				{#snippet unsavedConfirmationModal({
					diffDrawer,
					additionalExitAction,
					getInitialAndModifiedValues
				})}
					<UnsavedConfirmationModal
						{diffDrawer}
						{additionalExitAction}
						{getInitialAndModifiedValues}
					/>
				{/snippet}
			</AppEditor>
		</div>
	{/if}
{/key}
