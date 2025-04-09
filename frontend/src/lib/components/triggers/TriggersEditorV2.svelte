<script lang="ts">
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import TriggersTable from './TriggersTable.svelte'
	import RouteEditorWrapper from './http/RouteEditorWrapper.svelte'
	import CaptureWrapper from './CaptureWrapperV2.svelte'
	import type { Trigger } from './utils'

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

	let eventStreamType: 'kafka' | 'nats' | 'sqs' | 'mqtt' = 'kafka'
	let args: Record<string, any> = {}
	let canEdit = true
	let isEditing = false

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
	let selectedTrigger: Trigger | undefined = undefined

	// Handle trigger selection
	function handleSelectTrigger(event: CustomEvent<Trigger | undefined>) {
		selectedTrigger = event.detail
	}
</script>

<FlowCard {noEditor}>
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane class="px-4">
				<div class="flex flex-col h-full gap-2">
					<!-- Left Pane - Triggers List -->
					<div class="w-full flex-shrink-0 overflow-auto">
						<TriggersTable
							path={currentPath}
							{isFlow}
							on:select={handleSelectTrigger}
							{canEdit}
							bind:isEditing
						/>
					</div>

					<!-- Right Pane - Trigger Configuration -->
					<div class="flex-grow overflow-auto px-2 pb-4">
						<!-- Trigger configuration will go here -->
						{#if selectedTrigger}
							{#if selectedTrigger.type === 'routes'}
								<RouteEditorWrapper
									{selectedTrigger}
									{isFlow}
									{currentPath}
									on:update-config={({ detail }) => {
										args = detail
									}}
									{isEditing}
								/>
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
			<Pane class="px-4">
				{#if selectedTrigger && selectedTrigger?.type === 'routes'}
					<CaptureWrapper
						path={initialPath || fakeInitialPath}
						{isFlow}
						captureType={'http'}
						{hasPreprocessor}
						{canHavePreprocessor}
						{args}
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
