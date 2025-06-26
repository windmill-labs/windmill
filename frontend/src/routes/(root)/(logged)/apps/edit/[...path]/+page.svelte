<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import {
		AppService,
		type AppWithLastVersion,
		type AppWithLastVersionWDraft,
		DraftService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { cleanValueProperties, decodeState, type Value, clone} from '$lib/utils'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { untrack } from 'svelte'

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
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	afterNavigate(() => {
		if (nodraft) {
			let url = new URL($page.url.href)
			url.search = ''
			replaceState(url.toString(), $page.state)
		}
	})
	const initialState = nodraft ? undefined : localStorage.getItem(`app-${$page.params.path}`)
	let stateLoadedFromLocalStorage =
		initialState != undefined ? decodeState(initialState) : undefined

	async function loadApp(): Promise<void> {
		const app_w_draft = await AppService.getAppByPathWithDraft({
			path,
			workspace: $workspaceStore!
		})
		const app_w_draft_: AppWithLastVersionWDraft = clone(app_w_draft)
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

		if (stateLoadedFromLocalStorage) {
			const reloadAction = async () => {
				stateLoadedFromLocalStorage = undefined
				await loadApp()
				redraw++
			}
			const actions: ToastAction[] = []
			if (stateLoadedFromLocalStorage) {
				actions.push({
					label: 'Discard browser autosave and reload',
					callback: reloadAction
				})

				const draftOrDeployed = cleanValueProperties(savedApp.draft || savedApp)
				const urlScript = {
					...draftOrDeployed,
					value: stateLoadedFromLocalStorage
				}
				actions.push({
					label: 'Show diff',
					callback: async () => {
						diffDrawer?.openDrawer()
						diffDrawer?.setDiff({
							mode: 'simple',
							original: draftOrDeployed,
							current: urlScript,
							title: `${savedApp?.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
							button: { text: 'Discard autosave', onClick: reloadAction }
						})
					}
				})
			}

			sendUserToast('App restored from browser storage', false, actions)
			app_w_draft.value = stateLoadedFromLocalStorage
			app = app_w_draft
		} else if (app_w_draft.draft) {
			if (app_w_draft.summary !== undefined) {
				// backward compatibility for old drafts missing metadata
				app = {
					...app_w_draft,
					...app_w_draft.draft
				}
			} else {
				app = {
					...app_w_draft,
					value: app_w_draft.draft as any
				}
			}

			if (!app_w_draft.draft_only) {
				const reloadAction = () => {
					stateLoadedFromLocalStorage = undefined
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
		} else {
			app = app_w_draft
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
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
		if (savedApp.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'app',
				path: savedApp.path
			})
		}
		goto(`/apps/edit/${savedApp.path}`)
		await loadApp()
		redraw++
	}

	let diffDrawer: DiffDrawer | undefined = $state()

	function onRestore(ev: any) {
		sendUserToast('App restored from previous deployment')
		app = ev.detail
		const app_ = clone(app!)
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
				on:savedNewAppPath={(event) => {
					goto(`/apps/edit/${event.detail}`)
					if (app) {
						app.path = event.detail
					}
				}}
				on:restore={onRestore}
				summary={app.summary}
				app={app.value}
				newPath={app.path}
				path={$page.params.path}
				policy={app.policy}
				bind:savedApp
				{diffDrawer}
				version={app.versions ? app.versions[app.versions.length - 1] : undefined}
				newApp={false}
				replaceStateFn={(path) => replaceState(path, $page.state)}
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
