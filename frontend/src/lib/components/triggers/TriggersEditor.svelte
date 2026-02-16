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
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { Plus, BookOpen, ExternalLink } from 'lucide-svelte'
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
		SqsTriggerService,
		EmailTriggerService
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Alert from '../common/alert/Alert.svelte'
	import type { FlowEditorContext } from '../flows/types'

	interface Props {
		noEditor: boolean
		newItem?: boolean
		currentPath: string
		fakeInitialPath?: string
		runnableVersion?: string | undefined
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
		runnableVersion = undefined,
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

	const flowEditorContext = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	const chatInputEnabled = $derived(
		Boolean(flowEditorContext?.flowStore?.val?.value?.chat_input_enabled)
	)

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
			http: () => HttpTriggerService.deleteHttpTrigger,
			email: () => EmailTriggerService.deleteEmailTrigger
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
		} else if (triggerType === 'email') {
			await triggersState.fetchEmailTriggers(
				triggersCount,
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'nextcloud') {
			await triggersState.fetchNativeTriggers(
				triggersCount,
				'nextcloud',
				$workspaceStore,
				currentPath,
				isFlow,
				$userStore
			)
		} else if (triggerType === 'google') {
			await triggersState.fetchNativeTriggers(
				triggersCount,
				'google',
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
		console.log('handleUpdateDraftConfig', triggerIndex, newConfig, saveDisabled)
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

	let leftPaneWidth = $state(0)
</script>

<div bind:clientWidth={width} class="h-full w-full">
	{#if chatInputEnabled}
		<div class="p-2 pb-0">
			<Alert type="warning" title="Chat Input Mode Enabled" size="xs">
				This flow will only accept
				<span class="font-mono text-xs bg-surface-secondary px-1 rounded"> user_message</span>
				as input parameter.
			</Alert>
		</div>
	{/if}
	<FlowCard {noEditor} noHeader>
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full" bind:clientWidth={leftPaneWidth}>
					<!-- Left Pane - Triggers List -->
					{#if !useVerticalTriggerBar || !triggersState.selectedTrigger}
						<div
							class="overflow-auto p-2"
							style={triggersState.selectedTrigger
								? 'width: 350px; flex: 0 0 350px;'
								: 'flex: 1 1 350px; min-width: 350px;'}
						>
							<TriggersTable
								selectedTrigger={triggersState.selectedTriggerIndex}
								{onSelect}
								triggers={triggersState.triggers}
								{isEditor}
								onAddDraftTrigger={handleAddTrigger}
								onDeleteDraft={deleteTrigger}
								onReset={handleResetDraft}
								webhookToken={$triggersCount?.webhook_count}
								emailToken={$triggersCount?.default_email_count}
							/>
							{#if leftPaneWidth <= 800 && !triggersState.selectedTrigger}
								<div class="mt-6">
									{@render TriggerInfo()}
								</div>
							{/if}
						</div>
					{:else}
						<div class="p-2 flex flex-col gap-2 border-r justify-start">
							<AddTriggersButton
								onAddDraftTrigger={handleAddTrigger}
								class="w-fit h-fit"
								placement="right-start"
							>
								<Button
									variant="accent"
									btnClasses="h-8 w-8 p-0"
									nonCaptureEvent
									startIcon={{ icon: Plus }}
								/>
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

					{#if triggersState.selectedTrigger}
						<div
							class="flex-grow overflow-auto py-2 px-4 transition-all duration-200 ease-in-out"
							style="scrollbar-gutter: stable"
						>
							{#if loading}
								<div
									class="animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms] h-full w-full"
								></div>
							{:else}
								{#key [renderCount, triggersState.selectedTriggerIndex].join('-')}
									<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
										<TriggersWrapperV2
											selectedTrigger={triggersState.selectedTrigger}
											{isFlow}
											{initialPath}
											{fakeInitialPath}
											{currentPath}
											{runnableVersion}
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
											onEmailDomain={(domain) => {
												emailDomain = domain
											}}
										/>
									</div>
								{/key}
							{/if}
						</div>
					{:else if leftPaneWidth > 800}
						{@render TriggerInfo(true)}
					{/if}
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
								data={{ args, hash: !isFlow ? runnableVersion : undefined, emailDomain }}
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

{#snippet TriggerInfo(margin: boolean = false)}
	<div class="p-4 rounded bg-surface-tertiary grow min-w-[450px] h-fit {margin ? 'm-2' : ''}">
		<div class="flex gap-3">
			<BookOpen size={16} class="text-secondary mt-0.5 flex-shrink-0" />
			<div class="flex-1">
				<div class="text-2xs font-normal text-secondary mb-2">
					<strong class="font-semibold">Triggers</strong> automatically run your flow when events
					happen. Select one to configure it or create a new one.
					<a
						href="https://www.windmill.dev/docs/getting_started/triggers"
						target="_blank"
						class="whitespace-nowrap">Learn more <ExternalLink size={14} class="inline-block" /></a
					>
				</div>
				<ul class="text-2xs text-hint space-y-1 ml-2">
					<li class="flex">
						<span class="mr-2">•</span>
						<span>Default triggers: Webhooks, email, CLI - always exist, can't be disabled</span>
					</li>
					<li class="flex">
						<span class="mr-2">•</span>
						<span
							>Additional triggers: Setup advanced triggers, like HTTP routes, database changes,
							message queues, cloud events, and more</span
						>
					</li>
				</ul>
			</div>
		</div>
	</div>
{/snippet}
