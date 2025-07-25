<script lang="ts">
	import { AppService, DraftService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { cleanValueProperties, decodeState, type Value } from '$lib/utils'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'

	let files: Record<string, string> | undefined = undefined
	let runnables = {}
	let newPath = ''
	let lastVersion = 0
	let policy: any = {}
	let summary = ''

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
		| undefined = undefined
	let redraw = 0
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	afterNavigate(() => {
		if (nodraft) {
			let url = new URL($page.url.href)
			url.search = ''
			replaceState(url.toString(), $page.state)
		}
	})

	function extractRawApp(app: any) {
		runnables = app.value.runnables
		files = app.value.files
		summary = app.summary
		lastVersion = app.version
		policy = app.policy
		newPath = app.path
	}

	const initialState = nodraft ? undefined : localStorage.getItem(`rawapp-${$page.params.path}`)
	let stateLoadedFromLocalStorage =
		initialState != undefined ? decodeState(initialState) : undefined

	async function loadApp(): Promise<void> {
		const app_w_draft = await AppService.getAppByPathWithDraft({
			path,
			workspace: $workspaceStore!
		})
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
			app_w_draft.value = stateLoadedFromLocalStorage
			files = app_w_draft.value.files as any
			runnables = app_w_draft.value.runnables as any
			redraw += 1
		} else if (app_w_draft.draft) {
			extractRawApp(app_w_draft.draft)

			if (!app_w_draft.draft_only) {
				const reloadAction = () => {
					stateLoadedFromLocalStorage = undefined
					extractRawApp(app_w_draft)
					redraw++
				}

				const deployed = cleanValueProperties(app_w_draft as Value)
				const draft = cleanValueProperties({ files, runnables })
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
			extractRawApp(app_w_draft)
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
					goto(`/apps_raw/edit/${event.detail}`)
					newPath = event.detail
				}}
				on:restore={onRestore}
				initFiles={files}
				initRunnables={runnables}
				{summary}
				{newPath}
				path={$page.params.path}
				{policy}
				bind:savedApp
				{diffDrawer}
				version={lastVersion}
				newApp={false}
			/>
		</div>
	{/key}
{/if}
