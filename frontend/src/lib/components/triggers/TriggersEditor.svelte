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
		triggerTypeToCaptureKind,
		deleteTrigger,
		type TriggerType
	} from './utils'

	interface Props {
		noEditor: boolean
		newItem?: boolean
		currentPath: string
		fakeInitialPath?: string
		hash?: string | undefined
		args?: Record<string, any>
		initialPath: string
		isFlow: boolean
		canHavePreprocessor?: boolean
		hasPreprocessor?: boolean
		isDeployed?: boolean
		schema?: Record<string, any> | undefined
		noCapture?: boolean
		isEditor?: boolean
	}

	let {
		noEditor,
		newItem = false,
		currentPath,
		fakeInitialPath = '',
		hash = undefined,
		args = {},
		initialPath,
		isFlow,
		canHavePreprocessor = false,
		hasPreprocessor = false,
		isDeployed = false,
		schema = undefined,
		noCapture = false,
		isEditor = true
	}: Props = $props()

	let config: Record<string, any> = $state({})
	let width = $state(0)
	let emailDomain: string | undefined = $state(undefined)
	let isValid = $state(false)
	let renderCount = $state(0)

	const useVerticalTriggerBar = $derived(width < 1000)

	const {
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
		if (!trigger || trigger === -1) {
			return
		}

		const triggerType = $triggers[trigger].type
		//delete the trigger from the store
		$selectedTrigger = undefined
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

	function handleUpdateDraftConfig(
		triggerIndex: number | undefined,
		newConfig: Record<string, any>,
		saveDisabled: boolean
	) {
		if (triggerIndex && triggerIndex !== -1 && newConfig) {
			$triggers[triggerIndex].draftConfig = { ...newConfig, canSave: !saveDisabled }
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
</script>

<FlowCard {noEditor} noHeader bind:width>
	<Splitpanes horizontal>
		<Pane>
			<div class="flex flex-row h-full">
				<!-- Left Pane - Triggers List -->
				{#if !useVerticalTriggerBar}
					<div class="w-[350px] flex-shrink-0 overflow-auto pr-2 pl-4 pt-2 pb-2">
						{#key $triggers}
							<TriggersTable
								selectedTrigger={$selectedTrigger}
								{onSelect}
								triggers={$triggers}
								{isEditor}
								onAddDraftTrigger={handleAddTrigger}
								onDeleteDraft={deleteDraftTrigger}
								onReset={handleResetDraft}
							/>
						{/key}
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
						{#each $triggers as trigger, index}
							{#key renderCount}
								{#if $selectedTrigger === index}
									<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
										<TriggersWrapperV2
											selectedTrigger={trigger}
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
													handleUpdateDraftConfig($selectedTrigger, cfg, canSave)
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
								{/if}
							{/key}
						{/each}
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
</FlowCard>
