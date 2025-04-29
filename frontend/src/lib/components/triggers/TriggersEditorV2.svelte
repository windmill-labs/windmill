<script lang="ts">
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import TriggersTable from './TriggersTable.svelte'
	import RoutesPanel from './http/RoutesPanelV2.svelte'
	import CaptureWrapper from './CaptureWrapperV2.svelte'
	import { type Trigger } from './utils'
	import WebhooksPanel from './webhook/WebhooksPanelV2.svelte'
	import { triggerTypeToCaptureKind } from './utils'
	import { type CaptureTriggerKind } from '$lib/gen'
	import EmailTriggerPanel from '../details/EmailTriggerPanelV2.svelte'
	import PrimarySchedulePanel from './PrimarySchedulePanel.svelte'
	import SchedulePanel from '$lib/components/SchedulePanel.svelte'
	import {
		fetchHttpTriggers as fetchHttpTriggersUtil,
		fetchSchedules as fetchSchedulesUtil,
		fetchWebsocketTriggers,
		fetchPostgresTriggers,
		fetchKafkaTriggers,
		fetchNatsTriggers,
		fetchGcpTriggers,
		deleteDraft,
		addDraftTrigger
	} from './utils'
	import { workspaceStore } from '$lib/stores'
	import WebsocketTriggersPanel from './websocket/WebsocketTriggersPanelV2.svelte'
	import { fade } from 'svelte/transition'
	import PostgresTriggersPanel from './postgres/PostgresTriggersPanelV2.svelte'
	import KafkaTriggerPanel from './kafka/KafkaTriggerPanelV2.svelte'
	import NatsTriggerPanel from './nats/NatsTriggerPanelV2.svelte'
	import MqttTriggerPanel from './mqtt/MqttTriggerPanelV2.svelte'
	import SqsTriggerPanel from './sqs/SqsTriggerPanelV2.svelte'
	import { fetchMqttTriggers, fetchSqsTriggers } from './utils'
	import GcpTriggerPanel from './gcp/GcpTriggerPanelV2.svelte'
	import ScheduledPollPanel from './scheduled/ScheduledPollPanel.svelte'
	import TriggersBadgeV2 from '../graph/renderers/triggers/TriggersBadgeV2.svelte'
	import { twMerge } from 'tailwind-merge'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'

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

	let captureKind: CaptureTriggerKind | undefined = undefined

	$: if ($selectedTrigger) {
		captureKind = triggerTypeToCaptureKind($selectedTrigger.type)
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
		deleteDraft(triggers, trigger)
		if ($triggers.length > 0) {
			$selectedTrigger = $triggers[$triggers.length - 1]
		} else {
			$selectedTrigger = undefined
		}
	}

	$: updateEditTrigger($selectedTrigger)

	$: useVerticalTriggerBar = width < 800
</script>

<FlowCard {noEditor} title={'Triggers'} noHeader={useVerticalTriggerBar} bind:width>
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					{#if !useVerticalTriggerBar}
						<div class="w-[350px] flex-shrink-0 overflow-auto pr-2 pl-4">
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
								allwaysUseDropdown
							/>
						</div>
					{/if}

					<!-- TODO: Update triggersWrapper here -->
					<div
						class={twMerge(
							'flex-grow overflow-auto pl-2 pr-4 pb-4',
							useVerticalTriggerBar ? 'pl-4 pt-2' : ''
						)}
						style="scrollbar-gutter: stable"
					>
						{#if $selectedTrigger}
							{#key $selectedTrigger}
								<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
									{#if $selectedTrigger.type === 'http'}
										<RoutesPanel
											selectedTrigger={$selectedTrigger}
											{isFlow}
											path={initialPath || fakeInitialPath}
											edit={editTrigger === $selectedTrigger}
											on:update-config={({ detail }) => {
												config = detail
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchHttpTriggersUtil(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$userStore
												)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											{isDeployed}
											small={useVerticalTriggerBar}
										/>
									{:else if $selectedTrigger.type === 'webhook'}
										<WebhooksPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											{hash}
											token=""
											{args}
											scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
											{newItem}
										/>
									{:else if $selectedTrigger.type === 'email'}
										<EmailTriggerPanel
											token=""
											scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
											path={initialPath || fakeInitialPath}
											{isFlow}
											on:emailDomain={({ detail }) => {
												config.emailDomain = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'schedule' && $selectedTrigger.isPrimary}
										<PrimarySchedulePanel
											{schema}
											{isFlow}
											path={initialPath}
											{newItem}
											can_write={canWrite(currentPath, {}, $userStore)}
											on:update={async ({ detail }) => {
												await fetchSchedulesUtil(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$primarySchedule
												)
												if (
													(detail === 'save' || detail === 'delete') &&
													$selectedTrigger?.isDraft
												) {
													deleteDraft(triggers, $selectedTrigger)
													if (detail === 'delete') {
														$selectedTrigger = undefined
													}
												}
											}}
											isNewSchedule={$selectedTrigger.isDraft}
											{isDeployed}
										/>
									{:else if $selectedTrigger.type === 'schedule'}
										<SchedulePanel
											selectedTrigger={$selectedTrigger}
											{isFlow}
											path={initialPath}
											on:update={async ({ detail }) => {
												if ($selectedTrigger && $selectedTrigger.isDraft && detail?.path) {
													await fetchSchedulesUtil(
														triggers,
														$workspaceStore,
														currentPath,
														isFlow,
														$primarySchedule
													)
													$selectedTrigger.isDraft = false
													$selectedTrigger.path = detail.path
												}
											}}
										/>
									{:else if $selectedTrigger.type === 'websocket'}
										<WebsocketTriggersPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchWebsocketTriggers(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$userStore
												)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'kafka'}
										<KafkaTriggerPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchKafkaTriggers(triggers, $workspaceStore, currentPath, isFlow)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'postgres'}
										<PostgresTriggersPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchPostgresTriggers(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$userStore
												)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'nats'}
										<NatsTriggerPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchNatsTriggers(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$userStore
												)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'mqtt'}
										<MqttTriggerPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchMqttTriggers(
													triggers,
													$workspaceStore,
													currentPath,
													isFlow,
													$userStore
												)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'sqs'}
										<SqsTriggerPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchSqsTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'gcp'}
										<GcpTriggerPanel
											{isFlow}
											path={initialPath || fakeInitialPath}
											selectedTrigger={$selectedTrigger}
											edit={editTrigger === $selectedTrigger}
											{isDeployed}
											isEditor={true}
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											on:update={({ detail }) => {
												if ($selectedTrigger?.isDraft) {
													$selectedTrigger.isDraft = false
												}
												if ($selectedTrigger) {
													$selectedTrigger.path = detail
												}
												fetchGcpTriggers(triggers, $workspaceStore, currentPath, isFlow, $userStore)
											}}
											on:delete={() => {
												deleteDraftTrigger($selectedTrigger)
											}}
											on:update-config={({ detail }) => {
												config = detail
											}}
										/>
									{:else if $selectedTrigger.type === 'poll'}
										<ScheduledPollPanel />
									{:else if $selectedTrigger.isDraft}
										<h3 class="text-sm font-medium"
											>Configure new {$selectedTrigger.type} trigger</h3
										>
										<!-- New trigger configuration component would go here -->
									{:else}
										<h3 class="text-sm font-medium"
											>Configure trigger: {$selectedTrigger.path} ({$selectedTrigger.type})</h3
										>
										<!-- Existing trigger configuration component would go here -->
									{/if}
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
			{#if $selectedTrigger && $selectedTrigger.type !== 'schedule' && $selectedTrigger.type != 'poll'}
				<Pane>
					<div class="h-full w-full overflow-auto px-4" style="scrollbar-gutter: stable">
						{#if $selectedTrigger && $selectedTrigger?.type && captureKind}
							{#key captureKind}
								<div
									in:fade={{ duration: 100, delay: 100 }}
									out:fade={{ duration: 100 }}
									class="h-full w-full"
								>
									<CaptureWrapper
										path={initialPath || fakeInitialPath}
										{isFlow}
										captureType={captureKind}
										{hasPreprocessor}
										{canHavePreprocessor}
										args={{}}
										data={{ args, hash }}
										on:applyArgs
										on:updateSchema
										on:addPreprocessor
										on:testWithArgs
									/>
								</div>
							{/key}
						{/if}
					</div>
				</Pane>
			{/if}
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
