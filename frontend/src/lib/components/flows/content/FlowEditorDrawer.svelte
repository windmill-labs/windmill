<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink, Loader2 } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { emptySchema, type StateStore } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { initFlow } from '$lib/components/flows/flowStore.svelte'
	import type { FlowState } from '$lib/components/flows/flowState'
	import type { FlowEditorContext } from '../types'

	let flowEditorDrawer: Drawer | undefined = $state()

	const dispatch = createEventDispatcher()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	let opWs = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	export async function openDrawer(path: string, cb: () => void): Promise<void> {
		flowPath = path
		flow = undefined
		loading = true
		callback = cb
		flowEditorDrawer?.openDrawer?.()

		try {
			const backendFlow = await FlowService.getFlowByPath({
				workspace: opWs!,
				path
			})

			savedFlow = structuredClone(backendFlow) as Flow

			flow = backendFlow

			await initFlow(flow, flowStore, flowStateStore)
			loading = false
		} catch (error: any) {
			console.error('Failed to load flow:', error)
			loading = false
		}
	}

	let callback: (() => void) | undefined = undefined
	let flowPath: string = $state('')
	let flow: Flow | undefined = $state(undefined)
	let savedFlow: Flow | undefined = $state(undefined)
	let loading = $state(true)

	const flowStore: StateStore<Flow> = $state({
		val: {
			summary: '',
			value: { modules: [] },
			path: '',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}
	})
	const flowStateStore: StateStore<FlowState> = $state({ val: {} })

	let diffDrawer: DiffDrawer | undefined = $state()
</script>

<Drawer bind:this={flowEditorDrawer} size="100%">
	<DrawerContent
		title="Flow Editor - {flowPath}"
		noPadding
		fullScreen
		on:close={() => {
			flowEditorDrawer?.closeDrawer()
		}}
	>
		{#if loading}
			<div
				out:fade={{ duration: 200 }}
				class="absolute inset-0 center-center flex-col bg-surface text-primary border"
			>
				<Loader2 class="animate-spin" size={16} />
				<span class="text-xs mt-1">Loading flow...</span>
			</div>
		{:else if flow}
			<FlowBuilder
				{flowStore}
				{flowStateStore}
				initialPath={flowPath}
				autosaveWorkspace={opWs}
				newFlow={false}
				selectedId="settings-metadata"
				loading={false}
				bind:savedFlow
				{diffDrawer}
				customUi={{ topBar: { draft: false, extraDeployOptions: false } }}
				onDeploy={() => {
					callback?.()
					dispatch('save')
					flowEditorDrawer?.closeDrawer()
				}}
				onDetails={() => {
					callback?.()
					dispatch('save')
					flowEditorDrawer?.closeDrawer()
				}}
			/>
		{:else}
			<div class="flex items-center justify-center h-full">
				<span class="text-red-500">Failed to load flow</span>
			</div>
		{/if}
		{#snippet actions()}
			<Button
				variant="default"
				on:click={() => {
					window.open(`/flows/edit/${flowPath}`, '_blank', 'noopener,noreferrer')
					flowEditorDrawer?.closeDrawer()
				}}
				startIcon={{ icon: ExternalLink }}
			>
				Edit in new tab & close drawer
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<DiffDrawer bind:this={diffDrawer} isFlow />
