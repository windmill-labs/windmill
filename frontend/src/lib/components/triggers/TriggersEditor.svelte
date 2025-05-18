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
		type Trigger,
		triggerTypeToCaptureKind,
		deleteTrigger,
		type TriggerType
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
	let emailDomain: string | undefined = undefined
	let isValid = false
	let renderCount = 0

	const {
		simplifiedPoll,
		selectedTrigger: selectedTrigger,
		triggers,
		triggersCount
	} = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})

	// Handle trigger selection
	function onSelect(triggerIndex: number) {
		$selectedTrigger = triggerIndex
	}

	function deleteDraftTrigger(triggerIndex: number | undefined) {
		if (triggerIndex === undefined) {
			return
		}
		deleteTrigger(triggers, triggersCount, triggerIndex)

		// Select a new trigger if any exist
		if ($triggers.length > 0) {
			$selectedTrigger = $triggers.length - 1
		} else {
			$selectedTrigger = undefined
		}
	}

	async function handleUpdate(trigger: number | undefined, path: string) {
		if (!trigger) {
			return
		}

		const triggerType = $triggers[trigger].type
		//delete the trigger from the store
		deleteTrigger(triggers, triggersCount, trigger)

		if (triggerType === 'schedule') {
			await fetchSchedules(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				undefined,
				$userStore
			)
		} else if (triggerType === 'websocket') {
			await fetchWebsocketTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'postgres') {
			await fetchPostgresTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'kafka') {
			await fetchKafkaTriggers(triggers, triggersCount, $workspaceStore, currentPath, isFlow)
		} else if (triggerType === 'nats') {
			await fetchNatsTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'gcp') {
			await fetchGcpTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'sqs') {
			await fetchSqsTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'mqtt') {
			await fetchMqttTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'http') {
			await fetchHttpTriggers(
				triggers,
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		}
		$selectedTrigger = $triggers.findIndex((t) => t.path === path && t.type === triggerType)
	}

	function handleUpdateDraftConfig(newConfig: Record<string, any>, canSave: boolean) {
		if ($selectedTrigger && newConfig) {
			$triggers[$selectedTrigger].draftConfig = { ...newConfig, canSave }
		}
	}

	function handleResetDraft(trigger: number | undefined) {
		if (!trigger) {
			return
		}
		$triggers[trigger].draftConfig = undefined
		renderCount++
	}

	function handleAddTrigger(type: TriggerType) {
		const newTrigger = addDraftTrigger(
			triggers,
			triggersCount,
			type,
			type === 'schedule' ? initialPath : undefined
		)
		$selectedTrigger = newTrigger
	}

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
								{onSelect}
								triggers={$triggers}
								{isEditor}
								onAddDraftTrigger={handleAddTrigger}
								onDeleteDraft={deleteDraftTrigger}
								onReset={handleResetDraft}
							/>
						</div>
					{:else}
						<div class="p-2 flex flex-col gap-2 border-r">
							<AddTriggersButton
								onAddDraftTrigger={handleAddTrigger}
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
								small={false}
								vertical
								onSelect={(triggerIndex: number) => ($selectedTrigger = triggerIndex)}
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
							{@const selected = $triggers[$selectedTrigger]}
							{#key [selected.id || selected.path, renderCount].join('-')}
								<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
									<TriggersWrapperV2
										selectedTrigger={selected}
										{isFlow}
										{initialPath}
										{fakeInitialPath}
										{currentPath}
										{hash}
										{isDeployed}
										small={useVerticalTriggerBar}
										{args}
										{newItem}
										{schema}
										{isEditor}
										onDelete={() => {
											deleteDraftTrigger($selectedTrigger)
										}}
										onUpdate={(path) => {
											handleUpdate($selectedTrigger, path)
										}}
										onConfigChange={(cfg, canSave, updated) => {
											if (updated) {
												handleUpdateDraftConfig(cfg, canSave)
											}
										}}
										onCaptureConfigChange={(cfg, isValidConfig) => {
											config = cfg
											isValid = isValidConfig
										}}
										onReset={() => {
											handleResetDraft($selectedTrigger)
										}}
										on:email-domain={({ detail }) => {
											emailDomain = detail
										}}
									/>
								</div>
							{/key}
						{:else}
							<span class="text-sm text-tertiary text-center mx-auto mt-2"
								>{`Select a trigger from the ${useVerticalTriggerBar ? 'left toolbar' : 'table'} or a create a new one`}</span
							>
						{/if}
					</div>
				</div>
			</Pane>
			{#if $selectedTrigger && $triggers[$selectedTrigger].type && $triggers[$selectedTrigger].type !== 'schedule' && $triggers[$selectedTrigger].type != 'poll' && !noCapture}
				{@const captureKind = triggerTypeToCaptureKind($triggers[$selectedTrigger].type)}
				{#key captureKind}
					<Pane minSize={20} size={40}>
						<CaptureWrapper
							path={initialPath || fakeInitialPath}
							{isFlow}
							captureType={captureKind}
							{hasPreprocessor}
							{canHavePreprocessor}
							args={config}
							data={{ args, hash, emailDomain }}
							{isValid}
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
		{@const selected = $triggers.findIndex((t) => t.isPrimary)}
		<div class="px-4 py-2">
			{#if selected}
				<SchedulePanel
					{isFlow}
					path={initialPath || fakeInitialPath}
					selectedTrigger={selected}
					{isDeployed}
					defaultValues={$triggers[selected].draftConfig ??
						$triggers[selected].captureConfig ??
						undefined}
					newDraft={$triggers[selected].draftConfig === undefined}
					edit={editTrigger === selected}
					{schema}
					{isEditor}
					onDelete={() => {
						deleteDraftTrigger(selected)
					}}
					onUpdate={({ detail }) => {
						handleUpdate(selected, detail)
					}}
					onReset={() => {
						handleResetDraft(selected)
					}}
				/>
			{/if}
		</div>
	{/if}
</FlowCard>
