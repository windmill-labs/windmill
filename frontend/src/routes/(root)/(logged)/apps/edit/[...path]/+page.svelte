<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, AppWithLastVersion, DraftService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { cleanValueProperties, decodeState } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import { cloneDeep } from 'lodash'

	let app = undefined as (AppWithLastVersion & { draft_only?: boolean }) | undefined
	let savedApp:
		| {
				value: App
				draft?: any
				path: string
				summary: string
				policy: any
				draft_only?: boolean
		  }
		| undefined = undefined
	let redraw = 0
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	const initialState = nodraft ? undefined : localStorage.getItem(`app-${$page.params.path}`)
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	async function loadApp(): Promise<void> {
		const app_w_draft = await AppService.getAppByPathWithDraft({
			path,
			workspace: $workspaceStore!
		})
		const app_w_draft_ = cloneDeep(app_w_draft)
		savedApp = {
			summary: app_w_draft_.summary,
			value: app_w_draft_.value,
			path: app_w_draft_.path,
			policy: app_w_draft_.policy,
			draft_only: app_w_draft_.draft_only,
			draft:
				app_w_draft_.draft?.summary !== undefined // backward compatibility for old drafts missing metadata
					? app_w_draft_.draft
					: app_w_draft_.draft
					? {
							summary: app_w_draft_.summary,
							value: app_w_draft_.draft,
							path: app_w_draft_.path,
							policy: app_w_draft_.policy
					  }
					: undefined
		}

		if (stateLoadedFromUrl) {
			const reloadAction = async () => {
				stateLoadedFromUrl = undefined
				await loadApp()
				redraw++
			}
			const actions: ToastAction[] = []
			if (stateLoadedFromUrl) {
				actions.push({
					label: 'Discard URL stored autosave and reload',
					callback: reloadAction
				})

				const draftOrDeployed = cleanValueProperties(savedApp.draft || savedApp)
				const urlScript = {
					...draftOrDeployed,
					value: stateLoadedFromUrl
				}
				actions.push({
					label: 'Show diff',
					callback: async () => {
						diffDrawer.openDrawer()
						diffDrawer.setDiff({
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
			app_w_draft.value = stateLoadedFromUrl
			app = app_w_draft
		} else if (app_w_draft.draft) {
			app = (
				app_w_draft.draft.summary !== undefined // backward compatibility for old drafts missing metadata
					? app_w_draft.draft
					: {
							summary: app_w_draft.summary,
							value: app_w_draft.draft,
							path: app_w_draft.path,
							policy: app_w_draft.policy
					  }
			) as AppWithLastVersion
			if (!app_w_draft.draft_only) {
				const reloadAction = () => {
					stateLoadedFromUrl = undefined
					app = app_w_draft
					redraw++
				}

				const deployed = cleanValueProperties(app_w_draft)
				const draft = cleanValueProperties(app)
				sendUserToast('app loaded from latest saved draft', false, [
					{
						label: 'Discard draft and load from latest deployed version',
						callback: reloadAction
					},
					{
						label: 'Show diff',
						callback: async () => {
							diffDrawer.openDrawer()
							diffDrawer.setDiff({
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

	$: {
		if ($workspaceStore) {
			loadApp()
		}
	}

	async function restoreDraft() {
		if (!savedApp || !savedApp.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer.closeDrawer()
		goto(`/apps/edit/${savedApp.draft.path}`)
		await loadApp()
		redraw++
	}

	async function restoreDeployed() {
		if (!savedApp) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer.closeDrawer()
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

	let diffDrawer: DiffDrawer
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />

{#key redraw}
	{#if app}
		<div class="h-screen">
			<AppEditor
				on:restore={(e) => {
					sendUserToast('App restored from previous deployment')
					app = e.detail
					redraw++
				}}
				versions={app.versions}
				summary={app.summary}
				app={app.value}
				path={app.path}
				policy={app.policy}
				bind:savedApp
				{diffDrawer}
			/>
		</div>
	{/if}
{/key}
