<script lang="ts">
	import { Button } from '$lib/components/common'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import type { FlowModule, FlowStatusModule, Job } from '$lib/gen'
	import { Building, Repeat, Square, ArrowDown, GitBranch } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import { prettyLanguage } from '$lib/common'
	import { msToSec } from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import FlowJobsMenu from './FlowJobsMenu.svelte'
	import {
		isTriggerStep,
		type onSelectedIteration
	} from '$lib/components/graph/graphBuilder.svelte'
	import { checkIfParentLoop } from '$lib/components/flows/utils'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		moduleId: string
		mod: FlowModule
		insertable: boolean
		annotation?: string | undefined
		bgColor?: string
		bgHoverColor?: string
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
		waitingJob?: Job | undefined
		isOwner?: boolean
		type?: FlowStatusModule['type'] | undefined
		darkMode?: boolean
		skipped?: boolean
	}

	let {
		onSelectedIteration,
		moduleId,
		mod = $bindable(),
		insertable,
		annotation = undefined,
		bgColor = '',
		bgHoverColor = '',
		moving = undefined,
		duration_ms = undefined,
		retries = undefined,
		flowJobs,
		editMode = false,
		onSelect,
		onTestUpTo,
		onUpdateMock,
		onEditInput,
		waitingJob,
		isOwner = false,
		type,
		darkMode,
		skipped
	}: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const { flowStore } = getContext<FlowEditorContext | undefined>('FlowEditorContext') || {}

	const dispatch = createEventDispatcher<{
		delete: CustomEvent<MouseEvent>
		select: string
		newBranch: { id: string }
		move: { module: FlowModule } | undefined
	}>()

	let itemProps = $derived({
		selected: $selectedId === mod.id,
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
</script>

{#if mod}
	<div class="relative">
		{#if moving == mod.id}
			<div class="absolute z-10 right-20 top-0.5 center-center">
				<Button color="dark" on:click={() => dispatch('move')} size="xs" variant="border">
					Cancel move
				</Button>
			</div>
		{/if}

		{#if duration_ms}
			<div
				class={twMerge(
					'absolute z-10 right-0 -top-4 center-center text-tertiary text-2xs',
					editMode ? 'text-gray-400 dark:text-gray-500 text-2xs font-normal mr-2' : ''
				)}
			>
				{msToSec(duration_ms)}s
			</div>
		{/if}
		{#if annotation && annotation != ''}
			<div class="absolute z-10 left-0 -top-5 center-center text-tertiary">
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
					label={`${
						mod.summary || (mod.value.type == 'forloopflow' ? 'For loop' : 'While loop')
					}  ${mod.value.parallel ? '(parallel)' : ''} ${
						mod.value.skip_failures ? '(skip failures)' : ''
					}`}
					id={mod.id}
					on:changeId
					on:move
					on:delete
					on:pointerdown={() => onSelect(mod.id)}
					onUpdateMock={(mock) => {
						mod.mock = mock
						onUpdateMock?.({ id: mod.id, mock })
					}}
					{...itemProps}
					{bgColor}
					{bgHoverColor}
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
						<div>
							<Repeat size={16} />
						</div>
					{/snippet}
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchone'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					on:changeId
					on:delete
					on:move
					on:pointerdown={() => onSelect(mod.id)}
					{...itemProps}
					id={mod.id}
					label={mod.summary || 'Run one branch'}
					{bgColor}
					{bgHoverColor}
					{onTestUpTo}
				>
					{#snippet icon()}
						<div>
							<GitBranch size={16} />
						</div>
					{/snippet}
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchall'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					on:changeId
					on:delete
					on:move
					on:pointerdown={() => onSelect(mod.id)}
					id={mod.id}
					{...itemProps}
					label={mod.summary || `Run all branches${mod.value.parallel ? ' (parallel)' : ''}`}
					{bgColor}
					{bgHoverColor}
					{onTestUpTo}
				>
					{#snippet icon()}
						<div>
							<GitBranch size={16} />
						</div>
					{/snippet}
				</FlowModuleSchemaItem>
			{:else}
				<FlowModuleSchemaItem
					{retries}
					{editMode}
					on:changeId
					on:pointerdown={() => onSelect(mod.id)}
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
					{bgColor}
					{bgHoverColor}
					label={mod.summary ||
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
					{waitingJob}
					{isOwner}
					enableTestRun
					{type}
					{darkMode}
					{skipped}
				>
					{#snippet icon()}
						<div>
							{#if mod.value.type === 'rawscript'}
								<LanguageIcon lang={mod.value.language} width={16} height={16} />
							{:else if mod.summary == 'Terminate flow'}
								<Square size={16} />
							{:else if mod.value.type === 'identity'}
								<ArrowDown size={16} />
							{:else if mod.value.type === 'flow'}
								<BarsStaggered size={16} />
							{:else if mod.value.type === 'script'}
								{#if mod.value.path.startsWith('hub/')}
									<div>
										<IconedResourceType
											width="20px"
											height="20px"
											name={mod.value.path.split('/')[2]}
											silent={true}
										/>
									</div>
								{:else}
									<Building size={14} />
								{/if}
							{/if}
						</div>
					{/snippet}
				</FlowModuleSchemaItem>
			{/if}
		</div>
	</div>
{/if}
