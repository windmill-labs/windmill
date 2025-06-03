<script lang="ts">
	import { Button } from '$lib/components/common'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import type { FlowModule } from '$lib/gen'
	import { Building, Repeat, Square, ArrowDown, GitBranch } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import { prettyLanguage } from '$lib/common'
	import { msToSec } from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import FlowJobsMenu from './FlowJobsMenu.svelte'
	import { isTriggerStep } from '$lib/components/graph/graphBuilder'
	import { checkIfParentLoop, evalValue } from '$lib/components/flows/utils'
	import type { FlowEditorContext } from '$lib/components/flows/types'

	export let mod: FlowModule
	export let insertable: boolean
	export let annotation: string | undefined = undefined
	export let bgColor: string = ''
	export let bgHoverColor: string = ''
	export let moving: string | undefined = undefined
	export let duration_ms: number | undefined = undefined

	export let retries: number | undefined = undefined
	export let flowJobs:
		| {
				flowJobs: string[]
				selected: number
				flowJobsSuccess: (boolean | undefined)[]
				selectedManually: boolean | undefined
		  }
		| undefined
	export let editMode: boolean = false

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const { flowStore, testStepStore, flowStateStore } =
		getContext<FlowEditorContext | undefined>('FlowEditorContext') || {}

	const dispatch = createEventDispatcher<{
		delete: CustomEvent<MouseEvent>
		insert: {
			modules: FlowModule[]
			index: number
			detail: 'script' | 'forloop' | 'whileloop' | 'branchone' | 'branchall' | 'move'
			script?: { path: string; summary: string; hash: string | undefined }
		}
		select: string
		newBranch: { module: FlowModule }
		move: { module: FlowModule } | undefined
		selectedIteration: { index: number; id: string }
		updateMock: void
	}>()

	$: itemProps = {
		selected: $selectedId === mod.id,
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined || mod.stop_after_all_iters_if != undefined,
		skip: Boolean(mod.skip_if),
		suspend: Boolean(mod.suspend),
		sleep: Boolean(mod.sleep),
		cache: Boolean(mod.cache_ttl),
		mock: mod.mock,
		concurrency: Boolean(mod?.value?.['concurrent_limit'])
	}

	$: parentLoop = flowStore && $flowStore && mod ? checkIfParentLoop($flowStore, mod.id) : undefined

	function onDelete(event: CustomEvent<MouseEvent>) {
		dispatch('delete', event)
	}

	function getStepArgs() {
		if (!mod.id || !$flowStateStore?.[mod.id]?.schema?.properties || !$testStepStore) return

		return Object.fromEntries(
			Object.keys($flowStateStore?.[mod.id]?.schema?.properties ?? {}).map((k) => [
				k,
				evalValue(k, mod, $testStepStore, undefined, false)
			])
		)
	}

	let stepArgs: Record<string, any> | undefined = getStepArgs()
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
			<div class="absolute z-10 right-0 -top-4 center-center text-tertiary text-2xs">
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
					id={mod.id}
					on:selectedIteration={(e) => {
						dispatch('selectedIteration', e.detail)
					}}
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
					on:move={() => dispatch('move')}
					on:delete={onDelete}
					on:pointerdown={() => dispatch('select', mod.id)}
					on:updateMock={({ detail }) => {
						mod.mock = detail
						dispatch('updateMock')
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
				>
					<div slot="icon">
						<Repeat size={16} />
					</div>
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchone'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					on:changeId
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					on:pointerdown={() => dispatch('select', mod.id)}
					{...itemProps}
					id={mod.id}
					label={mod.summary || 'Run one branch'}
					{bgColor}
					{bgHoverColor}
				>
					<div slot="icon">
						<GitBranch size={16} />
					</div>
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchall'}
				<FlowModuleSchemaItem
					deletable={insertable}
					{editMode}
					on:changeId
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					on:pointerdown={() => dispatch('select', mod.id)}
					id={mod.id}
					{...itemProps}
					label={mod.summary || `Run all branches${mod.value.parallel ? ' (parallel)' : ''}`}
					{bgColor}
					{bgHoverColor}
				>
					<div slot="icon">
						<GitBranch size={16} />
					</div>
				</FlowModuleSchemaItem>
			{:else}
				<FlowModuleSchemaItem
					{retries}
					{editMode}
					on:changeId
					on:pointerdown={() => dispatch('select', mod.id)}
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					on:updateMock={({ detail }) => {
						mod.mock = detail
						dispatch('updateMock')
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
					{stepArgs}
				>
					<div slot="icon">
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
				</FlowModuleSchemaItem>
			{/if}
		</div>
	</div>
{/if}
