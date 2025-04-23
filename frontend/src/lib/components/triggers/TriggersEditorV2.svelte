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
		deleteDraft,
		addDraftTrigger
	} from './utils'
	import { workspaceStore } from '$lib/stores'
	import WebsocketTriggersPanel from './websocket/WebsocketTriggersPanelV2.svelte'
	import { fade } from 'svelte/transition'

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

	$: updateEditTrigger($selectedTrigger)
</script>

<FlowCard {noEditor} title="Triggers">
	{#if !$simplifiedPoll}
		<Splitpanes horizontal>
			<Pane>
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
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
								deleteDraft(triggers, detail.trigger)
								if ($triggers.length > 0) {
									$selectedTrigger = $triggers[$triggers.length - 1]
								}
							}}
							on:edit={({ detail }) => {
								editTrigger = detail
								if (JSON.stringify(detail) !== JSON.stringify($selectedTrigger)) {
									$selectedTrigger = detail
								}
							}}
						/>
					</div>

					<!-- TODO: Update triggersWrapper here -->
					<div class="flex-grow overflow-auto pl-2 pr-4 pb-4" style="scrollbar-gutter: stable">
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
											on:toggle-edit-mode={({ detail }) => {
												editTrigger = detail ? $selectedTrigger : undefined
											}}
											{isDeployed}
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
										/>
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
			{#if $selectedTrigger && $selectedTrigger.type !== 'schedule'}
				<Pane>
					<div class="h-full w-full overflow-auto px-4" style="scrollbar-gutter: stable">
						{#if $selectedTrigger && $selectedTrigger?.type && captureKind}
							{#key captureKind}
								<div in:fade={{ duration: 100, delay: 100 }} out:fade={{ duration: 100 }}>
									{#if captureKind === 'webhook'}
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
									{:else if captureKind === 'http'}
										<CaptureWrapper
											path={initialPath || fakeInitialPath}
											{isFlow}
											captureType={captureKind}
											{hasPreprocessor}
											{canHavePreprocessor}
											args={config}
											data={{ args }}
											on:applyArgs
											on:updateSchema
											on:addPreprocessor
											on:testWithArgs
										/>
									{:else if captureKind === 'websocket'}
										<CaptureWrapper
											path={initialPath || fakeInitialPath}
											{isFlow}
											captureType={captureKind}
											{hasPreprocessor}
											{canHavePreprocessor}
											args={config}
											data={{ args }}
											on:applyArgs
											on:updateSchema
											on:addPreprocessor
											on:testWithArgs
										/>
									{:else if captureKind === 'email'}
										<CaptureWrapper
											path={initialPath || fakeInitialPath}
											{isFlow}
											captureType={captureKind}
											{hasPreprocessor}
											{canHavePreprocessor}
											args={{}}
											data={{ emailDomain: config.emailDomain }}
											on:applyArgs
											on:updateSchema
											on:addPreprocessor
											on:testWithArgs
										/>
									{/if}
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
