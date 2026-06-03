<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import {
		AppService,
		type AppWithLastVersion,
		type AppWithLastVersionWDraft,
		DraftService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'

	let app = $state(undefined as (AppWithLastVersion & { value: any }) | undefined)
	let savedApp:
		| {
				value: App
				draft?: any
				path: string
				summary: string
				policy: any
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a reload) bail at the next checkpoint after their
	 * captured token no longer matches. */
	let loadAppToken = 0
	async function loadApp(): Promise<void> {
		const tok = ++loadAppToken
		let app_w_draft: AppWithLastVersionWDraft
		try {
			app_w_draft = await AppService.getAppByPathWithDraft({
				path: page.params.path ?? '',
				workspace: $workspaceStore!
			})
		} catch (e) {
			// No deployed app at this path: it may be a never-deployed item that
			// lives only in the draft table. Load it from there.
			const draft = await DraftService.getDraft({
				workspace: $workspaceStore!,
				kind: 'app',
				path: page.params.path ?? ''
			}).catch(() => undefined)
			if (tok !== loadAppToken) return
			if (!draft?.value) throw e
			// The app draft value has shape { value, path, summary, policy, custom_path }.
			const dv = draft.value as {
				value: any
				path: string
				summary: string
				policy: any
				custom_path?: string
			}
			// No deployed version → savedApp mirrors the draft, so the builder
			// treats a deploy as "create".
			savedApp = {
				summary: dv.summary,
				value: dv.value as App,
				path: dv.path,
				policy: dv.policy,
				draft: dv,
				custom_path: dv.custom_path
			}
			app = { ...dv } as AppWithLastVersion & { value: any }
			return
		}
		if (tok !== loadAppToken) return
		const app_w_draft_: AppWithLastVersionWDraft = structuredClone(stateSnapshot(app_w_draft))
		savedApp = {
			summary: app_w_draft_.summary,
			value: app_w_draft_.value as App,
			path: app_w_draft_.path,
			policy: app_w_draft_.policy,
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

		// The editor works off the backend DB draft when present, otherwise the
		// deployed version. AppEditor seeds its own in-memory draft handle from
		// this value; edits sync back to the DB.
		app = app_w_draft.draft
			? app_w_draft.summary !== undefined
				? ({ ...app_w_draft, ...app_w_draft.draft } as AppWithLastVersion & { value: any })
				: ({ ...app_w_draft, value: app_w_draft.draft } as AppWithLastVersion & { value: any })
			: app_w_draft
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
