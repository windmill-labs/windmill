<script lang="ts">
	import { canWrite } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import TriggersTable from './TriggersTable.svelte'
	import CaptureWrapper from './CaptureWrapperV2.svelte'
	import { type Trigger } from './utils'
	import { triggerTypeToCaptureKind } from './utils'
	import PrimarySchedulePanel from './PrimarySchedulePanel.svelte'
	import { fade } from 'svelte/transition'
	import TriggersBadgeV2 from '../graph/renderers/triggers/TriggersBadgeV2.svelte'
	import { twMerge } from 'tailwind-merge'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import TriggersWrapperV2 from './TriggersWrapperV2.svelte'
	import {
		fetchHttpTriggers,
		fetchSchedules,
		fetchWebsocketTriggers,
		fetchPostgresTriggers,
		fetchKafkaTriggers,
		fetchNatsTriggers,
		fetchGcpTriggers,
		fetchSqsTriggers,
		fetchMqttTriggers,
		addDraftTrigger,
		deleteDraft,
		updateDraftTriggerConfig
	} from './utils'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let fakeInitialPath: string
	export let hash: string | undefined = undefined
	export let args: Record<string, any> = {}
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let isDeployed: boolean = false

	let config: Record<string, any> = {}
	let editTrigger: Trigger | undefined = undefined
	let useVerticalTriggerBar = true
	let width = 0

	const {
		simplifiedPoll,
		selectedTriggerV2: selectedTrigger,
		triggers,
		primarySchedule
	} = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})

	// Handle trigger selection
	function handleSelectTrigger(event: CustomEvent<Trigger>) {
		$selectedTrigger = event.detail
	}

	function updateEditTrigger(trigger: Trigger | undefined) {
		if (editTrigger !== trigger) {
			editTrigger = undefined
		}
	}

	function deleteDraftTrigger(trigger: Trigger | undefined) {
		if (!trigger) {
			return
		}

		if ('id' in trigger && trigger.id) {
			deleteDraft(triggers, trigger.id)
		}

		// Select a new trigger if any exist
		if ($triggers.length > 0) {
			$selectedTrigger = $triggers[$triggers.length - 1]
		} else {
			$selectedTrigger = undefined
		}
	}

	function handleUpdate(trigger: Trigger | undefined, path: string) {
		if (!trigger) {
			return
		}
		if (trigger.isDraft) {
			trigger.isDraft = false
		}
		if (trigger) {
			trigger.path = path
		}
		if (trigger.type === 'schedule') {
			fetchSchedules(triggers, $workspaceStore, currentPath, isFlow, $primarySchedule)
		} else if (trigger.type === 'websocket') {
			fetchWebsocketTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'postgres') {
			fetchPostgresTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'kafka') {
			fetchKafkaTriggers(triggers, $workspaceStore, currentPath, isFlow)
		} else if (trigger.type === 'nats') {
			fetchNatsTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'gcp') {
			fetchGcpTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'sqs') {
			fetchSqsTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'mqtt') {
			fetchMqttTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'http') {
			fetchHttpTriggers(triggers, $workspaceStore, currentPath, isFlow)
		}
	}

	function handleUpdateConfig(newConfig: Record<string, any>) {
		// Update the config for the current trigger in our draft trigger store
		if ($selectedTrigger && 'id' in $selectedTrigger && $selectedTrigger.isDraft) {
			updateDraftTriggerConfig(triggers, $selectedTrigger.id as string, newConfig)
		}

		// Also maintain the config for the current component
		config = newConfig
	}

	$: updateEditTrigger($selectedTrigger)
	$: useVerticalTriggerBar = width < 1000
</script>

<FlowCard {noEditor} noHeader bind:width>
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					{#if !useVerticalTriggerBar}
						<div class="w-[350px] flex-shrink-0 overflow-auto pr-2 pl-4 pt-2">
							<TriggersTable
								selectedTrigger={$selectedTrigger}
								on:select={handleSelectTrigger}
								triggers={$triggers}
								on:addDraftTrigger={({ detail }) => {
									const newTrigger = addDraftTrigger(triggers, detail)
									$selectedTrigger = newTrigger
								}}
								on:deleteDraft={({ detail }) => {
									deleteDraftTrigger(detail.trigger)
								}}
								on:edit={({ detail }) => {
									editTrigger = detail
									if (JSON.stringify(detail) !== JSON.stringify($selectedTrigger)) {
										$selectedTrigger = detail
									}
								}}
							/>
						</div>
					{:else}
						<div class="p-2 flex flex-col gap-2 bg-surface-secondary">
							<AddTriggersButton
								on:addDraftTrigger={({ detail }) => {
									const newTrigger = addDraftTrigger(triggers, detail)
									$selectedTrigger = newTrigger
								}}
								class="w-fit h-fit"
								placement="right-start"
							>
								<Button size="xs" nonCaptureEvent btnClasses="p-2 w-fit" wrapperClasses="p-0">
									<Plus size="14" />
								</Button>
							</AddTriggersButton>
							<TriggersBadgeV2
								showOnlyWithCount={false}
								path={initialPath || fakeInitialPath}
								{newItem}
								isFlow
								selected={true}
								triggers={$triggers}
								small={false}
								on:select={({ detail }) => ($selectedTrigger = detail)}
							/>
						</div>
					{/if}

					<div
						class={twMerge(
							'flex-grow overflow-auto pl-2 pr-4 pb-4 pt-2',
							useVerticalTriggerBar ? 'pl-4 pt-2' : ''
						)}
						style="scrollbar-gutter: stable"
					>
						{#if $selectedTrigger}
							{#key $selectedTrigger}
								<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
									<TriggersWrapperV2
										selectedTrigger={$selectedTrigger}
										{isFlow}
										{initialPath}
										{fakeInitialPath}
										{currentPath}
										edit={editTrigger === $selectedTrigger}
										{hash}
										{isDeployed}
										small={useVerticalTriggerBar}
										{args}
										{newItem}
										{schema}
										on:update-config={({ detail }) => {
											handleUpdateConfig(detail)
										}}
										on:delete={() => {
											deleteDraftTrigger($selectedTrigger)
										}}
										on:toggle-edit-mode={({ detail }) => {
											editTrigger = detail ? $selectedTrigger : undefined
										}}
										on:update={({ detail }) => {
											handleUpdate($selectedTrigger, detail)
										}}
									/>
								</div>
							{/key}
						{:else}
							<span class="text-sm text-tertiary text-center mx-auto mt-2"
								>Select a trigger from the table or add a new one</span
							>
						{/if}
					</div>
				</div>
			</Pane>
			{#if $selectedTrigger && $selectedTrigger.type && $selectedTrigger.type !== 'schedule' && $selectedTrigger.type != 'poll'}
				{@const captureKind = triggerTypeToCaptureKind($selectedTrigger.type)}
				{#key captureKind}
					<Pane>
						<CaptureWrapper
							path={initialPath || fakeInitialPath}
							{isFlow}
							captureType={captureKind}
							{hasPreprocessor}
							{canHavePreprocessor}
							args={config}
							data={{ args, hash }}
							on:applyArgs
							on:updateSchema
							on:addPreprocessor
							on:testWithArgs
						/>
					</Pane>
				{/key}
			{/if}
		</Splitpanes>
	{:else}
		<div class="px-4 pb-2">
			<PrimarySchedulePanel
				{schema}
				{isFlow}
				path={initialPath}
				{newItem}
				can_write={canWrite(currentPath, {}, $userStore)}
				isNewSchedule={false}
				{isDeployed}
			/>
		</div>
	{/if}
</FlowCard>
