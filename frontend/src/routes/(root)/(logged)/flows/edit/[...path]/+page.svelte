<script lang="ts">
	import { FlowService, type Flow, DraftService } from '$lib/gen'

	import { page } from '$app/stores'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		decodeArgs,
		decodeState,
		emptySchema,
		orderedJsonStringify
	} from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { goto } from '$app/navigation'
	import { writable } from 'svelte/store'
	import type { FlowState } from '$lib/components/flows/flowState'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { cloneDeep } from 'lodash'

	let nodraft = $page.url.searchParams.get('nodraft')
	const initialState = nodraft ? undefined : localStorage.getItem(`flow-${$page.params.path}`)
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined
	const initialArgs = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = undefined

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	export const flowStore = writable<Flow>({
		summary: '',
		value: { modules: [] },
		path: '',
		edited_at: '',
		edited_by: '',
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	})
	const flowStateStore = writable<FlowState>({})

	let loading = false

	let selectedId: string = 'settings-metadata'

	let nobackenddraft = false
	async function loadFlow(): Promise<void> {
		loading = true
		let flow: Flow
		let statePath = stateLoadedFromUrl?.path
		if (stateLoadedFromUrl != undefined && statePath == $page.params.path) {
			savedFlow = await FlowService.getFlowByPathWithDraft({
				workspace: $workspaceStore!,
				path: statePath
			})

			const draftOrDeployed = cleanValueProperties(savedFlow?.draft || savedFlow)
			const urlScript = cleanValueProperties(stateLoadedFromUrl.flow)
			flow = stateLoadedFromUrl.flow
			const reloadAction = () => {
				stateLoadedFromUrl = undefined
				goto(`/flows/edit/${statePath}`)
				loadFlow()
			}
			if (orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(urlScript)) {
				reloadAction()
			} else {
				sendUserToast('Flow loaded from browser storage', false, [
					{
						label: 'Discard browser stored autosave and reload',
						callback: reloadAction
					},
					{
						label: 'Show diff',
						callback: async () => {
							diffDrawer.openDrawer()
							diffDrawer.setDiff({
								mode: 'simple',
								original: draftOrDeployed,
								current: urlScript,
								title: `${savedFlow?.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
								button: { text: 'Discard autosave', onClick: reloadAction }
							})
						}
					}
				])
			}
		} else {
			const flowWithDraft = await FlowService.getFlowByPathWithDraft({
				workspace: $workspaceStore!,
				path: $page.params.path
			})
			savedFlow = {
				...cloneDeep(flowWithDraft),
				draft: flowWithDraft.draft
					? {
							...cloneDeep(flowWithDraft.draft),
							path: flowWithDraft.draft.path ?? flowWithDraft.path // backward compatibility for old drafts missing path
					  }
					: undefined
			} as Flow & {
				draft?: Flow
			}
			if (flowWithDraft.draft != undefined && !nobackenddraft) {
				flow = flowWithDraft.draft
				if (!flowWithDraft.draft_only) {
					const deployed = cleanValueProperties(flowWithDraft)
					const draft = cleanValueProperties(flow)
					const reloadAction = async () => {
						stateLoadedFromUrl = undefined
						await DraftService.deleteDraft({
							workspace: $workspaceStore!,
							kind: 'flow',
							path: flow.path
						})
						nobackenddraft = true
						loadFlow()
					}
					sendUserToast('flow loaded from latest saved draft', false, [
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
				flow = flowWithDraft
			}
		}

		await initFlow(flow, flowStore, flowStateStore)
		loading = false
		selectedId = stateLoadedFromUrl?.selectedId ?? $page.url.searchParams.get('selected')
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}

	let diffDrawer: DiffDrawer

	async function restoreDraft() {
		if (!savedFlow || !savedFlow.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer.closeDrawer()
		goto(`/flows/edit/${savedFlow.draft.path}`)
		loadFlow()
	}

	async function restoreDeployed() {
		if (!savedFlow) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer.closeDrawer()
		if (savedFlow.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'flow',
				path: savedFlow.path
			})
		}
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" />

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />
<FlowBuilder
	on:deploy={(e) => {
		goto(`/flows/get/${e.detail}?workspace=${$workspaceStore}`)
	}}
	on:details={(e) => {
		goto(`/flows/get/${e.detail}?workspace=${$workspaceStore}`)
	}}
	{flowStore}
	{flowStateStore}
	initialPath={$page.params.path}
	newFlow={false}
	{selectedId}
	{initialArgs}
	{loading}
	bind:savedFlow
	{diffDrawer}
/>
