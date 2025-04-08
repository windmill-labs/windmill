<script lang="ts">
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import TriggersTable from './TriggersTable.svelte'
	import RouteEditorInner from './http/RouteEditorInner.svelte'
	import CaptureWrapper from './CaptureWrapperV2.svelte'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let fakeInitialPath: string
	export let hash: string | undefined = undefined
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let args: Record<string, any> = {}

	let eventStreamType: 'kafka' | 'nats' | 'sqs' | 'mqtt' = 'kafka'
	let routeEditor: RouteEditorInner | null = null

	const { selectedTrigger: contextSelectedTrigger, simplifiedPoll } =
		getContext<TriggerContext>('TriggerContext')

	$: {
		if (
			$contextSelectedTrigger === 'kafka' ||
			$contextSelectedTrigger === 'nats' ||
			$contextSelectedTrigger === 'sqs' ||
			$contextSelectedTrigger === 'mqtt'
		) {
			eventStreamType = $contextSelectedTrigger
		}
	}

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})

	// State to track selected trigger
	let selectedTrigger: { path: string; type: string; isDraft?: boolean } | null = null

	// Handle trigger selection
	function handleSelectTrigger(
		event: CustomEvent<{ path: string; type: string; isDraft?: boolean }>
	) {
		selectedTrigger = event.detail
	}

	$: selectedTrigger?.type === 'routes' &&
		routeEditor &&
		openRouteEditor(selectedTrigger.path, isFlow, selectedTrigger.isDraft ?? false)

	function openRouteEditor(path: string, isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, currentPath)
		} else {
			routeEditor?.openEdit(path, isFlow)
		}
	}
</script>

<FlowCard {noEditor} title="Triggers">
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					<div class="w-1/3 min-w-[280px] max-w-[320px] border-r flex-shrink-0 overflow-auto px-2">
						<TriggersTable
							path={currentPath}
							{isFlow}
							{selectedTrigger}
							on:select={handleSelectTrigger}
						/>
					</div>

					<!-- Right Pane - Trigger Configuration -->
					<div class="flex-grow overflow-auto px-2">
						<!-- Trigger configuration will go here -->
						{#if selectedTrigger}
							{#if selectedTrigger.type === 'routes'}
								<RouteEditorInner useDrawer={false} bind:this={routeEditor} hideTarget />
							{:else if selectedTrigger.isDraft}
								<h3 class="text-sm font-medium">Configure new {selectedTrigger.type} trigger</h3>
								<!-- New trigger configuration component would go here -->
							{:else}
								<h3 class="text-sm font-medium"
									>Configure trigger: {selectedTrigger.path} ({selectedTrigger.type})</h3
								>
								<!-- Existing trigger configuration component would go here -->
							{/if}
						{:else}
							<div class="flex h-full items-center justify-center text-tertiary">
								<p>Select a trigger from the list or add a new one</p>
							</div>
						{/if}
					</div>
				</div>
			</Pane>
			<Pane>
				{#if selectedTrigger && selectedTrigger?.type === 'routes'}
					<CaptureWrapper
						path={initialPath || fakeInitialPath}
						{isFlow}
						captureType={'http'}
						{hasPreprocessor}
						{canHavePreprocessor}
						on:applyArgs
						on:updateSchema
						on:addPreprocessor
						on:testWithArgs
					/>
				{/if}
			</Pane>
		</Splitpanes>
	{:else}
		<div class="px-4 pb-2">
			<RunPageSchedules
				{schema}
				{isFlow}
				path={initialPath}
				{newItem}
				can_write={canWrite(currentPath, {}, $userStore)}
			/>
		</div>
	{/if}
</FlowCard>
