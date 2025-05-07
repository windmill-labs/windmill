<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import TriggersTable from './TriggersTable.svelte'
	import CaptureWrapper from './CaptureWrapper.svelte'
	import { fade } from 'svelte/transition'
	import TriggersBadge from '../graph/renderers/triggers/TriggersBadge.svelte'
	import { twMerge } from 'tailwind-merge'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import TriggersWrapperV2 from './TriggersWrapper.svelte'
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
		updateDraftConfig,
		isEqual,
		type Trigger,
		triggerTypeToCaptureKind,
		setCaptureConfig,
		deleteTrigger
	} from './utils'
	import SchedulePanel from '../SchedulePanel.svelte'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let fakeInitialPath: string = ''
	export let hash: string | undefined = undefined
	export let args: Record<string, any> = {}
	export let initialPath: string
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let isDeployed: boolean = false
	export let schema: Record<string, any> | undefined = undefined
	export let noCapture: boolean = false
	export let isEditor: boolean = true

	let config: Record<string, any> = {}
	let editTrigger: Trigger | undefined = undefined
	let useVerticalTriggerBar = true
	let width = 0

	const {
		simplifiedPoll,
		selectedTrigger: selectedTrigger,
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

	async function handleUpdate(trigger: Trigger | undefined, path: string) {
		if (!trigger) {
			return
		}
		//delete the trigger from the store
		deleteTrigger(triggers, trigger)

		if (trigger.isDraft) {
			trigger.isDraft = false
		}
		if (trigger) {
			trigger.path = path
		}

		if (trigger.type === 'schedule') {
			await fetchSchedules(triggers, $workspaceStore, currentPath, isFlow, $primarySchedule)
		} else if (trigger.type === 'websocket') {
			await fetchWebsocketTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'postgres') {
			await fetchPostgresTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'kafka') {
			await fetchKafkaTriggers(triggers, $workspaceStore, currentPath, isFlow)
		} else if (trigger.type === 'nats') {
			await fetchNatsTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'gcp') {
			await fetchGcpTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'sqs') {
			await fetchSqsTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'mqtt') {
			await fetchMqttTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
		} else if (trigger.type === 'http') {
			await fetchHttpTriggers(triggers, $workspaceStore, currentPath, isFlow)
		}
		$selectedTrigger = trigger
	}

	function handleUpdateDraftConfig(newConfig: Record<string, any>) {
		// Update the config for the current trigger in our draft trigger store
		if ($selectedTrigger && newConfig) {
			updateDraftConfig(triggers, $selectedTrigger, newConfig)
		}
	}

	function handleResetDraft(trigger: Trigger | undefined) {
		if (!trigger) {
			return
		}
		updateDraftConfig(triggers, trigger, undefined)
		if ($selectedTrigger && isEqual(trigger, $selectedTrigger)) {
			$selectedTrigger.draftConfig = undefined
		}
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
						<div class="w-[350px] flex-shrink-0 overflow-auto pr-2 pl-4 pt-2 pb-2">
							<TriggersTable
								selectedTrigger={$selectedTrigger}
								on:select={handleSelectTrigger}
								triggers={$triggers}
								{isEditor}
								on:addDraftTrigger={({ detail }) => {
									const newTrigger = addDraftTrigger(
										triggers,
										detail,
										detail === 'schedule' ? initialPath : undefined
									)
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
								on:reset={({ detail }) => {
									handleResetDraft(detail)
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
							<TriggersBadge
								showOnlyWithCount={false}
								path={initialPath || fakeInitialPath}
								{newItem}
								isFlow
								selected={true}
								triggers={$triggers}
								small={false}
								vertical
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
										{isEditor}
										on:update-config={({ detail }) => {
											config = detail
											if ($selectedTrigger && $selectedTrigger.id) {
												setCaptureConfig(triggers, $selectedTrigger.id, detail)
											}
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
										on:save-draft={({ detail }) => {
											handleUpdateDraftConfig(detail.cfg)
										}}
										on:reset={() => {
											handleResetDraft($selectedTrigger)
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
			{#if $selectedTrigger && $selectedTrigger.type && $selectedTrigger.type !== 'schedule' && $selectedTrigger.type != 'poll' && !noCapture}
				{@const captureKind = triggerTypeToCaptureKind($selectedTrigger.type)}
				{#key captureKind}
					<Pane minSize={20} size={40}>
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
		{@const selected = $triggers.find((t) => t.isPrimary)}
		<div class="px-4 py-2">
			{#if selected}
				<SchedulePanel
					{isFlow}
					path={initialPath || fakeInitialPath}
					selectedTrigger={selected}
					{isDeployed}
					defaultValues={selected.draftConfig ?? selected.captureConfig ?? undefined}
					newDraft={selected.draftConfig === undefined}
					edit={editTrigger === selected}
					{schema}
					on:update-config
					on:update
					on:save-draft
					on:reset
					on:toggle-edit-mode
					on:delete
				/>
			{/if}
		</div>
	{/if}
</FlowCard>
