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
		deleteDraft,
		addDraftTrigger
	} from './utils'
	import { workspaceStore } from '$lib/stores'

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
			<Pane class="px-4">
				<div class="flex flex-row h-full">
					<!-- Left Pane - Triggers List -->
					<div class="w-[350px] flex-shrink-0 overflow-auto pr-2">
						<TriggersTable
							selectedTrigger={$selectedTrigger}
							on:select={handleSelectTrigger}
							triggers={$triggers}
							on:addDraftTrigger={({ detail }) => {
								const newTrigger = addDraftTrigger(triggers, detail, !!$primarySchedule)
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
								$selectedTrigger = detail
							}}
						/>
					</div>

					<!-- TODO: Update triggersWrapper here -->
					{#if $selectedTrigger}
						<div class="flex-grow overflow-auto px-2 pb-4 transition-all duration-200 ease-in-out">
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
										fetchHttpTriggersUtil(triggers, $workspaceStore, currentPath, isFlow)

										if (detail) {
											deleteDraft(triggers, $selectedTrigger)
											if ($triggers.length > 0) {
												$selectedTrigger = $triggers[$triggers.length - 1]
											}
										}
									}}
									on:toggle-edit-mode={({ detail }) => {
										editTrigger = detail ? $selectedTrigger : undefined
									}}
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
										if ((detail === 'save' || detail === 'delete') && $selectedTrigger?.isDraft) {
											deleteDraft(triggers, $selectedTrigger)
											if (detail === 'delete') {
												$selectedTrigger = undefined
											}
										}
									}}
									isNewSchedule={$selectedTrigger.isDraft}
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
							{:else if $selectedTrigger.isDraft}
								<h3 class="text-sm font-medium">Configure new {$selectedTrigger.type} trigger</h3>
								<!-- New trigger configuration component would go here -->
							{:else}
								<h3 class="text-sm font-medium"
									>Configure trigger: {$selectedTrigger.path} ({$selectedTrigger.type})</h3
								>
								<!-- Existing trigger configuration component would go here -->
							{/if}
						</div>
					{:else}
						<span class="text-sm text-tertiary text-center mx-auto mt-2"
							>Select a trigger from the table or add a new one</span
						>
					{/if}
				</div>
			</Pane>
			{#if $selectedTrigger && $selectedTrigger.type !== 'schedule'}
				<Pane class="px-4">
					{#if $selectedTrigger && $selectedTrigger?.type && captureKind}
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
					{/if}
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
