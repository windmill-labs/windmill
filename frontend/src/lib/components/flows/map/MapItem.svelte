<script lang="ts">
	import { Button } from '$lib/components/common'
	import type { FlowModule, Job } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import FlowModuleIcon from '../FlowModuleIcon.svelte'
	import { prettyLanguage } from '$lib/common'
	import { msToSec } from '$lib/utils'
	import FlowJobsMenu from './FlowJobsMenu.svelte'
	import {
		isTriggerStep,
		type onSelectedIteration
	} from '$lib/components/graph/graphBuilder.svelte'
	import { checkIfParentLoop } from '$lib/components/flows/utils.svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { twMerge } from 'tailwind-merge'
	import type { FlowNodeState } from '$lib/components/graph'
	import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'
	import { getGraphContext } from '$lib/components/graph/graphContext'

	interface Props {
		moduleId: string
		mod: FlowModule
		insertable: boolean
		moduleAction: ModuleActionInfo | undefined
		annotation?: string | undefined
		nodeState?: FlowNodeState
		moving?: string | undefined
		duration_ms?: number | undefined
		retries?: number | undefined
		flowJobs:
			| {
					flowJobs: string[]
					selected: number
					flowJobsSuccess: (boolean | undefined)[]
					selectedManually: boolean | undefined
			  }
			| undefined
		editMode?: boolean
		onSelectedIteration: onSelectedIteration
		onSelect: (id: string | FlowModule) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onUpdateMock?: (detail: {
			id: string
			mock: { enabled: boolean; return_value?: unknown }
		}) => void
		onEditInput?: (moduleId: string, key: string) => void
		flowJob?: Job | undefined
		isOwner?: boolean
		maximizeSubflow?: () => void
	}

	let {
		onSelectedIteration,
		moduleId,
		mod = $bindable(),
		insertable,
		moduleAction = undefined,
		annotation = undefined,
		nodeState,
		moving = undefined,
		duration_ms = undefined,
		retries = undefined,
		flowJobs,
		editMode = false,
		onSelect,
		onTestUpTo,
		onUpdateMock,
		onEditInput,
		flowJob,
		isOwner = false,
		maximizeSubflow
	}: Props = $props()

	const { selectionManager } = getGraphContext()

	const flowEditorContext = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	const { flowStore } = flowEditorContext || {}

	const dispatch = createEventDispatcher<{
		delete: CustomEvent<MouseEvent>
		select: string
		newBranch: { id: string }
		move: { module: FlowModule } | undefined
	}>()

	let itemProps = $derived({
		selected: selectionManager && selectionManager.isNodeSelected(mod.id),
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined || mod.stop_after_all_iters_if != undefined,
		skip: Boolean(mod.skip_if),
		suspend: Boolean(mod.suspend),
		sleep: Boolean(mod.sleep),
		cache: Boolean(mod.cache_ttl),
		mock: mod.mock,
		concurrency: Boolean(mod?.value?.['concurrent_limit'])
	})

	let parentLoop = $derived(
		flowStore?.val && mod ? checkIfParentLoop(flowStore.val, mod.id) : undefined
	)

	function handlePointerDown(e: CustomEvent<PointerEvent>) {
		// Only handle left clicks (button 0)
		if (e.detail.button === 0) {
			onSelect(mod.id)
		}
	}
</script>

{#if mod}
	<div class="relative">
		{#if moving == mod.id}
			<div class="absolute z-10 right-20 top-0.5 center-center">
				<Button variant="accent" on:click={() => dispatch('move')} size="xs" destructive
					>Cancel move</Button
				>
			</div>
		{/if}

		{#if duration_ms}
			<div
				class={twMerge(
					'absolute z-5 right-0 -top-4 center-center text-primary text-2xs',
					editMode ? 'text-gray-400 dark:text-gray-500 text-2xs font-normal mr-2 right-16' : ''
				)}
			>
				{msToSec(duration_ms)}s
			</div>
		{/if}
		{#if annotation && annotation != ''}
			<div
				class={twMerge(
					'absolute z-10 left-0 -top-5 center-center text-primary',
					editMode ? '-top-4 text-gray-400 dark:text-gray-500 text-xs font-normal' : ''
				)}
			>
				{annotation}
			</div>
		{/if}
		{#if flowJobs && !insertable && (mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow')}
			<div class="absolute right-8 z-50 -top-5">
				<FlowJobsMenu
					{moduleId}
					id={mod.id}
					{onSelectedIteration}
					flowJobsSuccess={flowJobs.flowJobsSuccess}
					flowJobs={flowJobs.flowJobs}
					selected={flowJobs.selected}
					selectedManually={flowJobs.selectedManually}
				/>
			</div>
		{/if}

		<div class={moving == mod.id ? 'opacity-50' : ''}>
			{#if mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					{moduleAction}
					label={`${
						mod.summary || (mod.value.type == 'forloopflow' ? 'For loop' : 'While loop')
					}  ${mod.value.parallel ? '(parallel)' : ''} ${
						mod.value.skip_failures ? '(skip failures)' : ''
					} ${mod.value.squash ? '(squash)' : ''}`}
					id={mod.id}
					on:changeId
					on:move
					on:delete
					on:pointerdown={handlePointerDown}
					onUpdateMock={(mock) => {
						mod.mock = mock
						onUpdateMock?.({ id: mod.id, mock })
					}}
					{...itemProps}
					{nodeState}
					warningMessage={mod?.value?.type === 'forloopflow' &&
					mod?.value?.iterator?.type === 'javascript' &&
					mod?.value?.iterator?.expr === ''
						? 'Iterator expression is empty'
						: ''}
					alwaysShowOutputPicker={!mod.id.startsWith('subflow:')}
					loopStatus={{ type: 'self', flow: mod.value.type }}
					{onTestUpTo}
				>
					{#snippet icon()}
						<FlowModuleIcon module={mod} />
					{/snippet}
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchone'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					{moduleAction}
					on:changeId
					on:delete
					on:move
					on:pointerdown={handlePointerDown}
					{...itemProps}
					id={mod.id}
					label={mod.summary || 'Run one branch'}
					{nodeState}
					{onTestUpTo}
				>
					{#snippet icon()}
						<FlowModuleIcon module={mod} />
					{/snippet}
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchall'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					{moduleAction}
					on:changeId
					on:delete
					on:move
					on:pointerdown={handlePointerDown}
					id={mod.id}
					{...itemProps}
					label={mod.summary || `Run all branches${mod.value.parallel ? ' (parallel)' : ''}`}
					{nodeState}
					{onTestUpTo}
				>
					{#snippet icon()}
						<FlowModuleIcon module={mod} />
					{/snippet}
				</FlowModuleSchemaItem>
			{:else}
				<FlowModuleSchemaItem
					{retries}
					{editMode}
					{moduleAction}
					on:changeId
					on:pointerdown={handlePointerDown}
					on:delete
					on:move
					onUpdateMock={(mock) => {
						console.log('onUpdateMock', mock)

						mod.mock = mock
						onUpdateMock?.({ id: mod.id, mock })
					}}
					deletable={insertable}
					id={mod.id}
					{...itemProps}
					modType={mod.value.type}
					{nodeState}
					label={mod.summary ||
						(mod.value.type === 'aiagent' ? 'AI Agent' : undefined) ||
						(mod.id === 'preprocessor'
							? 'Preprocessor'
							: mod.id.startsWith('failure')
								? 'Error Handler'
								: undefined) ||
						(`path` in mod.value ? mod.value.path : undefined) ||
						(mod.value.type === 'rawscript'
							? `Inline ${prettyLanguage(mod.value.language)}`
							: 'To be defined')}
					path={`path` in mod.value ? mod.value.path : ''}
					isTrigger={isTriggerStep(mod)}
					alwaysShowOutputPicker={!mod.id.startsWith('subflow:') && mod.id !== 'preprocessor'}
					loopStatus={parentLoop ? { type: 'inside', flow: parentLoop.type } : undefined}
					inputTransform={mod.value.type !== 'identity' ? mod.value.input_transforms : undefined}
					{onTestUpTo}
					{onEditInput}
					{flowJob}
					{isOwner}
					enableTestRun
					{maximizeSubflow}
				>
					{#snippet icon()}
						{@const size =
							mod.value.type === 'script' && mod.value.path.startsWith('hub/')
								? 20
								: mod.value.type === 'script'
									? 14
									: 16}
						<FlowModuleIcon module={mod} {size} />
					{/snippet}
				</FlowModuleSchemaItem>
			{/if}
		</div>
	</div>
{/if}
