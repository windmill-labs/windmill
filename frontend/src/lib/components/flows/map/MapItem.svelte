<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import FlowBranchOneMap from './FlowBranchOneMap.svelte'
	import FlowBranchAllMap from './FlowBranchAllMap.svelte'

	export let mod: FlowModule
	export let index: number
	export let color: 'blue' | 'orange' | 'indigo' = 'blue'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher<{ delete: CustomEvent<MouseEvent>; insert: void }>()

	$: itemProps = {
		selected: $selectedId === mod.id,
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined,
		suspend: Boolean(mod.suspend)
	}

	function onDelete(event: CustomEvent<CustomEvent<MouseEvent>>) {
		dispatch('delete', event.detail)
	}
</script>

{#if mod}
	<InsertModuleButton on:click={() => dispatch('insert')} />
	{#if mod.value.type === 'forloopflow'}
		<li class="w-full">
			<FlowModuleSchemaItem
				deletable
				label={mod.summary || 'For loop'}
				id={mod.id}
				on:delete={onDelete}
				on:click={() => select(mod.id)}
				{...itemProps}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
			</FlowModuleSchemaItem>
			<div class="flex flex-row w-full">
				<div class="w-8 shrink-0 line" />
				<div class="grow my-4 overflow-auto">
					<div class="w-full pr-1">
						<FlowModuleSchemaMap bind:modules={mod.value.modules} color="orange" />
					</div>
				</div>
			</div>
		</li>
	{:else if mod.value.type === 'branchone'}
		<li>
			<FlowModuleSchemaItem
				deletable
				on:delete={onDelete}
				on:click={() => select(mod.id)}
				{...itemProps}
				id={mod.id}
				label={mod.summary || 'Run one branch'}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
			</FlowModuleSchemaItem>
			<FlowBranchOneMap bind:module={mod} />
		</li>
	{:else if mod.value.type === 'branchall'}
		<li>
			<FlowModuleSchemaItem
				deletable
				on:delete={onDelete}
				on:click={() => select(mod.id)}
				id={mod.id}
				{...itemProps}
				label={mod.summary || 'Run all branches'}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
			</FlowModuleSchemaItem>
			<FlowBranchAllMap bind:module={mod} />
		</li>
	{:else}
		<li>
			<FlowModuleSchemaItem
				on:click={() => select(mod.id)}
				on:delete={onDelete}
				{color}
				deletable
				id={mod.id}
				{...itemProps}
				label={mod.summary ||
					(`path` in mod.value ? mod.value.path : undefined) ||
					(mod.value.type === 'rawscript' ? `Inline ${mod.value.language}` : 'To be defined')}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
			</FlowModuleSchemaItem>
		</li>
	{/if}
{/if}

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
