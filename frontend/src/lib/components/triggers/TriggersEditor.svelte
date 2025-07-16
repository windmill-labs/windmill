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
		triggerTypeToCaptureKind,
		type TriggerType,
		CLOUD_DISABLED_TRIGGER_TYPES,
		type Trigger
	} from './utils'
	import { isCloudHosted } from '$lib/cloud'
	import {
		ScheduleService,
		WebsocketTriggerService,
		PostgresTriggerService,
		KafkaTriggerService,
		NatsTriggerService,
		MqttTriggerService,
		HttpTriggerService,
		GcpTriggerService,
		SqsTriggerService
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

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
		onDeployTrigger?: (trigger: Trigger) => void
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
		isEditor = true,
		onDeployTrigger
	}: Props = $props()

	let config: Record<string, any> = $state({})
	let width = $state(0)
	let emailDomain: string | undefined = $state(undefined)
	let isValid = $state(false)
	let renderCount = $state(0)
	let loading = $state(false)

	const useVerticalTriggerBar = $derived(width < 1000)
	const { triggersState, triggersCount } = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})

	// Handle trigger selection
	function onSelect(triggerIndex: number) {
		triggersState.selectedTriggerIndex = triggerIndex
	}

	let deletingTrigger = $state<number | undefined>(undefined)
	async function deleteDeployedTrigger(triggerIndex: number) {
		const { type: triggerType, path: triggerPath } = triggersState.triggers[triggerIndex]
		if (!triggerPath) {
			return
		}

		const deleteHandlers = {
			schedule: () => ScheduleService.deleteSchedule,
			websocket: () => WebsocketTriggerService.deleteWebsocketTrigger,
			postgres: () => PostgresTriggerService.deletePostgresTrigger,
			kafka: () => KafkaTriggerService.deleteKafkaTrigger,
			nats: () => NatsTriggerService.deleteNatsTrigger,
			gcp: () => GcpTriggerService.deleteGcpTrigger,
			sqs: () => SqsTriggerService.deleteSqsTrigger,
			mqtt: () => MqttTriggerService.deleteMqttTrigger,
			http: () => HttpTriggerService.deleteHttpTrigger
		}

		const deleteHandler = deleteHandlers[triggerType as keyof typeof deleteHandlers]
		if (deleteHandler && deletingTrigger !== triggerIndex) {
			deletingTrigger = triggerIndex
			try {
				await deleteHandler()({
					workspace: $workspaceStore ?? '',
					path: triggerPath ?? ''
				})
				sendUserToast(`Successfully deleted ${triggerType} trigger: ${triggerPath}`)
			} catch (error) {
				sendUserToast(error.body || error.message, true)
			} finally {
				deletingTrigger = undefined
			}
		}
	}

	async function deleteTrigger(triggerIndex: number | undefined) {
		if (triggerIndex === undefined) {
			return
		}
		// If the trigger is deployed, delete the trigger from the db
		if (
			!triggersState.triggers[triggerIndex].isDraft &&
			triggersState.triggers[triggerIndex].path
		) {
			await deleteDeployedTrigger(triggerIndex)
		}

		triggersState.deleteTrigger(triggersCount, triggerIndex)
		triggersState.selectedTriggerIndex = triggersState.triggers.length - 1
	}

	async function handleUpdate(trigger: number | undefined, path: string) {
		if (!trigger || trigger === -1) {
			return
		}

		const { type: triggerType, id: triggerId, path: triggerPath } = triggersState.triggers[trigger]
		loading = true

		triggersState.selectedTriggerIndex = undefined
		triggersState.deleteTrigger(triggersCount, trigger)

		if (triggerType === 'schedule') {
			await triggersState.fetchSchedules(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				undefined,
				$userStore
			)
		} else if (triggerType === 'websocket') {
			await triggersState.fetchWebsocketTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'postgres') {
			await triggersState.fetchPostgresTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'kafka') {
			await triggersState.fetchKafkaTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'nats') {
			await triggersState.fetchNatsTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'gcp') {
			await triggersState.fetchGcpTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'sqs') {
			await triggersState.fetchSqsTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'mqtt') {
			await triggersState.fetchMqttTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'http') {
			await triggersState.fetchHttpTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		}

		triggersState.selectedTriggerIndex = triggersState.triggers.findIndex(
			(t) => t.path === path && t.type === triggerType
		)
		loading = false

		onDeployTrigger?.({ type: triggerType, id: triggerId, path: triggerPath })
	}

	function handleUpdateDraftConfig(
		triggerIndex: number | undefined,
		newConfig: Record<string, any>,
		saveDisabled: boolean
	) {
		if (triggerIndex && triggerIndex !== -1 && newConfig) {
			triggersState.setDraftConfig(triggerIndex, { ...newConfig, canSave: !saveDisabled })
		}
	}

	function handleResetDraft(trigger: number | undefined) {
		if (!trigger) {
			return
		}
		triggersState.setDraftConfig(trigger, undefined)
		renderCount++
	}

	function handleAddTrigger(type: TriggerType) {
		const newTrigger = triggersState.addDraftTrigger(
			triggersCount,
			type,
			type === 'schedule' ? initialPath : undefined
		)
		triggersState.selectedTriggerIndex = newTrigger
	}

	const cloudDisabled = $derived(
		CLOUD_DISABLED_TRIGGER_TYPES.includes(triggersState.selectedTrigger?.type ?? '') &&
			isCloudHosted()
	)
</script>

<div bind:clientWidth={width} class="h-full w-full">
	<FlowCard {noEditor} noHeader>
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					{#if !useVerticalTriggerBar}
						<div class="w-[350px] flex-shrink-0 overflow-auto pr-2 pl-4 pt-2 pb-2">
							<TriggersTable
								selectedTrigger={triggersState.selectedTriggerIndex}
								{onSelect}
								triggers={triggersState.triggers}
								{isEditor}
								onAddDraftTrigger={handleAddTrigger}
								onDeleteDraft={deleteTrigger}
								onReset={handleResetDraft}
								webhookToken={$triggersCount?.webhook_count}
								emailToken={$triggersCount?.email_count}
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
								onSelect={(triggerIndex: number) => {
									triggersState.selectedTriggerIndex = triggerIndex
								}}
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
						{#if loading}
							<div
								class="animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms] h-full w-full"
							></div>
						{:else if triggersState.selectedTrigger}
							{#key [renderCount, triggersState.selectedTriggerIndex].join('-')}
								<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
									<TriggersWrapperV2
										selectedTrigger={triggersState.selectedTrigger}
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
											deleteTrigger(triggersState.selectedTriggerIndex)
										}}
										onUpdate={(path) => {
											handleUpdate(triggersState.selectedTriggerIndex, path)
										}}
										onConfigChange={(cfg, canSave, updated) => {
											if (updated) {
												handleUpdateDraftConfig(triggersState.selectedTriggerIndex, cfg, canSave)
											}
										}}
										onCaptureConfigChange={(cfg, isValidConfig) => {
											config = cfg
											isValid = isValidConfig
										}}
										onReset={() => {
											handleResetDraft(triggersState.selectedTriggerIndex)
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
			{#if !cloudDisabled && triggersState.selectedTrigger && triggersState.selectedTrigger.type !== 'schedule' && triggersState.selectedTrigger.type != 'poll' && !noCapture}
				{@const captureKind = triggersState.selectedTrigger
					? triggerTypeToCaptureKind(triggersState.selectedTrigger.type)
					: undefined}
				{#if captureKind}
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
								triggerDeployed={!triggersState.selectedTrigger.isDraft}
								on:applyArgs
								on:updateSchema
								on:addPreprocessor
								on:testWithArgs
							/>
						</Pane>
					{/key}
				{/if}
			{/if}
		</Splitpanes>
	</FlowCard>
</div>
