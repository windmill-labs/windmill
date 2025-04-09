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
	import { triggerIconMap, type Trigger } from './utils'
	import { Star } from 'lucide-svelte'
	import WebhooksWrapper from './webhook/WebhooksWrapper.svelte'
	import { triggerTypeToCaptureKind } from './utils'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let fakeInitialPath: string
	//export let hash: string | undefined = undefined
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let args: Record<string, any> = {}
	let triggersTable: TriggersTable | null = null

	const { simplifiedPoll } = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})

	// State to track selected trigger
	let selectedTrigger: Trigger | null = null

	// Handle trigger selection
	function handleSelectTrigger(event: CustomEvent<Trigger>) {
		selectedTrigger = event.detail
	}
</script>

<FlowCard {noEditor} title="Triggers">
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane class="px-4">
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					<div class="w-1/3 min-w-[280px] max-w-[320px] flex-shrink-0 overflow-auto pr-2">
						<TriggersTable
							path={currentPath}
							{isFlow}
							{selectedTrigger}
							on:select={handleSelectTrigger}
							bind:this={triggersTable}
						/>
					</div>

					<!-- Right Pane - Trigger Configuration -->
					<div class="flex-grow overflow-auto px-2">
						<!-- Trigger configuration will go here -->
						{#if selectedTrigger}
							{#if selectedTrigger.type === 'http'}
								<RouteEditorWrapper
									{selectedTrigger}
									{isFlow}
									{currentPath}
									{header}
									on:update-config={({ detail }) => {
										args = detail
									}}
									on:update={({ detail }) => {
										triggersTable?.fetchHttpTriggers()
										if (detail) {
											triggersTable?.deleteDraft('routes')
										}
									}}
								/>
							{:else if selectedTrigger.type === 'webhook'}
								<WebhooksWrapper isFlow={isFlow ?? false} path={currentPath} token={''} />
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
								<p>Select a trigger or add a new one</p>
							</div>
						{/if}
					</div>
				</div>
			</Pane>
			<Pane class="px-4">
				{#if selectedTrigger && selectedTrigger?.type && triggerTypeToCaptureKind(selectedTrigger.type)}
					<CaptureWrapper
						path={initialPath || fakeInitialPath}
						{isFlow}
						captureType={triggerTypeToCaptureKind(selectedTrigger.type)}
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

{#snippet header()}
	<div class="flex flex-row items-center text-secondary h-[32px]">
		<div class="text-center p-2">
			<div class="flex justify-center items-center">
				<svelte:component
					this={triggerIconMap[selectedTrigger?.type ?? '']}
					size={24}
					class={selectedTrigger?.isDraft ? 'text-frost-400' : 'text-tertiary'}
				/>

				{#if selectedTrigger?.isPrimary}
					<Star size={10} class="absolute -mt-3 ml-3 text-yellow-400" />
				{/if}
			</div>
		</div>
		<div class="py-2 px-2">
			<div class="flex items-center">
				<span class={selectedTrigger?.isDraft ? 'text-frost-400 italic' : ''}>
					{selectedTrigger?.isDraft
						? `New ${selectedTrigger?.type.replace(/s$/, '')} trigger`
						: selectedTrigger?.path}
				</span>

				{#if selectedTrigger?.isPrimary}
					<span
						class="ml-2 text-2xs bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-100 px-1.5 py-0.5 rounded"
					>
						Primary
					</span>
				{/if}

				{#if selectedTrigger?.isDraft}
					<span
						class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded"
					>
						Draft
					</span>
				{/if}
			</div>
		</div>
	</div>
{/snippet}
