<script lang="ts">
	import { run } from 'svelte/legacy'

	import { AppService, DraftService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		readFieldsRecursively,
		type Value
	} from '$lib/utils'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { page } from '$app/state'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import { UserDraft } from '$lib/userDraft.svelte'

	type RawAppDraft = {
		files: Record<string, string>
		runnables: Record<string, any>
		data: RawAppData
		summary: string
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

	const draftHandle = UserDraft.use<RawAppDraft>('raw_app', path)

	// Persist the bundle whenever any of the four pieces of state changes.
	$effect(() => {
		if (!files) return
		readFieldsRecursively(files)
		readFieldsRecursively(runnables)
		readFieldsRecursively(data)
		void summary
		draftHandle.draft = { files, runnables, data, summary }
	})

	let nodraft = page.url.searchParams.get('nodraft')

	afterNavigate(() => {
		if (nodraft) {
			let url = new URL(page.url.href)
			url.search = ''
			replaceState(url.toString(), page.state)
		}
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

		const backendSource: any = app_w_draft.draft ? app_w_draft.draft : app_w_draft
		const localDraft = UserDraft.get<RawAppDraft>('raw_app', path)
		const backendBundle: RawAppDraft = {
			files: backendSource.value?.files ?? {},
			runnables: backendSource.value?.runnables ?? {},
			data:
				backendSource.value?.data ??
				(backendSource.value?.datatables
					? { ...DEFAULT_DATA, tables: backendSource.value.datatables }
					: { ...DEFAULT_DATA }),
			summary: backendSource.summary ?? ''
		}

		if (
			localDraft != undefined &&
			orderedJsonStringify(cleanValueProperties(localDraft)) !==
				orderedJsonStringify(cleanValueProperties(backendBundle))
		) {
			const reloadAction = async () => {
				UserDraft.remove('raw_app', path)
				await loadApp()
				redraw++
			}
			const deployed = cleanValueProperties(app_w_draft as Value)
			const local = { ...deployed, value: localDraft }
			sendUserToast('App restored from local autosave', false, [
				{
					label: 'Discard local autosave and reload',
					callback: reloadAction
				},
				{
					label: 'Show diff',
					callback: async () => {
						diffDrawer?.openDrawer()
						diffDrawer?.setDiff({
							mode: 'simple',
							original: deployed,
							current: local,
							title: `${app_w_draft.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
							button: { text: 'Discard autosave', onClick: reloadAction }
						})
					}
				}
			])
			runnables = localDraft.runnables
			data = localDraft.data
			summary = localDraft.summary
			policy = app_w_draft.policy
			newPath = app_w_draft.path
			files = localDraft.files
		} else {
			if (localDraft != undefined) UserDraft.remove('raw_app', path)
			extractRawApp(backendSource)

			if (app_w_draft.draft && !app_w_draft.draft_only) {
				const reloadAction = () => {
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
		if ($workspaceStore) {
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
				{policy}
				bind:savedApp
				{diffDrawer}
				newApp={false}
			/>
		</div>
	{/key}
{/if}
