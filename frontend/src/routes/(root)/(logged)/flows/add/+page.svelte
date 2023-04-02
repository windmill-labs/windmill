<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import type { FlowState } from '$lib/components/flows/flowState'
	import { importFlowStore, initFlow } from '$lib/components/flows/flowStore'
	import { FlowService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { decodeState, emptySchema, sendUserToast } from '$lib/utils'
	import { writable } from 'svelte/store'

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')
	const initialState = hubId || templatePath || nodraft ? undefined : localStorage.getItem('flow')

	let selectedId: string = 'settings-metadata'
	let loading = false

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

	async function loadFlow() {
		loading = true
		let flow: Flow = {
			path: '',
			summary: '',
			value: { modules: [] },
			edited_by: '',
			edited_at: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}

		let state = initialState ? decodeState(initialState) : undefined
		if ($importFlowStore) {
			flow = $importFlowStore
			$importFlowStore = undefined
			sendUserToast('Flow loaded from JSON')
		} else if (!templatePath && !hubId && state) {
			sendUserToast('Flow restored from draft')
			flow = state.flow
			state?.selectedId && (selectedId = state?.selectedId)
		} else {
			if (templatePath) {
				const template = await FlowService.getFlowByPath({
					workspace: $workspaceStore!,
					path: templatePath
				})
				Object.assign(flow, template)
				const oldPath = flow.path.split('/')
				flow.path = `u/${$userStore?.username.split('@')[0]}/${oldPath[oldPath.length - 1]}_fork`
				flow = flow
				goto('?', { replaceState: true })
				selectedId = 'settings-metadata'
			} else if (hubId) {
				const hub = await FlowService.getHubFlowById({ id: Number(hubId) })
				delete hub['comments']
				flow.path = `u/${$userStore?.username}/flow_${hubId}`
				Object.assign(flow, hub.flow)
				flow = flow
				goto('?', { replaceState: true })
				selectedId = 'settings-metadata'
			}
		}
		await initFlow(flow, flowStore, flowStateStore)
		loading = false
	}

	loadFlow()

	$dirtyStore = true
</script>

<div id="monaco-widgets-root" class="monaco-editor" style="z-index: 999;" />

<FlowBuilder {flowStore} {flowStateStore} {selectedId} {loading} />
