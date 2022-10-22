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
	const dispatch = createEventDispatcher()

	$: itemProps = {
		selectedId: $selectedId === mod.id,
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined,
		suspend: Boolean(mod.suspend)
	}
</script>

{#if mod}
	<InsertModuleButton on:click={() => dispatch('insert')} />
	{#if mod.value.type === 'forloopflow'}
		<li>
			<FlowModuleSchemaItem deletable on:delete on:click={() => select(mod.id)} {...itemProps}>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="truncate block w-full text-xs">
					<span>{mod.summary || 'For loop'}</span>
				</div>
			</FlowModuleSchemaItem>
			<div class="flex text-xs">
				<div class="line mr-2" />
				<div class="w-full my-2">
					<FlowModuleSchemaMap bind:modules={mod.value.modules} color="orange" />
				</div>
			</div>
		</li>
	{:else if mod.value.type === 'branchone'}
		<li>
			<FlowModuleSchemaItem deletable on:delete on:click={() => select(mod.id)} {...itemProps}>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="truncate block w-full text-xs">
					<span>{mod.summary || 'Branches'}</span>
				</div>
			</FlowModuleSchemaItem>
			<FlowBranchOneMap bind:module={mod} />
		</li>
	{:else if mod.value.type === 'branchall'}
		<li>
			<FlowModuleSchemaItem deletable on:delete on:click={() => select(mod.id)} {...itemProps}>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="truncate block w-full text-xs">
					<span>{mod.summary || 'Branches'}</span>
				</div>
			</FlowModuleSchemaItem>
			<FlowBranchAllMap bind:module={mod} />
		</li>
	{:else}
		<li>
			<FlowModuleSchemaItem
				on:click={() => select(mod.id)}
				on:delete
				{color}
				deletable
				{...itemProps}
				id={mod.id}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="w-full truncate block text-xs">
					<span>
						{mod.summary ||
							(`path` in mod.value ? mod.value.path : undefined) ||
							(mod.value.type === 'rawscript' ? `Inline ${mod.value.language}` : 'Select a script')}
					</span>
				</div>
			</FlowModuleSchemaItem>
		</li>
	{/if}
{/if}
