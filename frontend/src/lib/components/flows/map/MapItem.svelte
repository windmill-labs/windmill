<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import FlowBranchOneMap from './FlowBranchOneMap.svelte'
	import FlowBranchAllMap from './FlowBranchAllMap.svelte'
	import {
		faBuilding,
		faCode,
		faCodeBranch,
		faLongArrowDown,
		faRepeat
	} from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	export let mod: FlowModule

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher<{
		delete: CustomEvent<MouseEvent>
		insert: 'script' | 'forloop' | 'branchone' | 'branchall'
	}>()

	$: itemProps = {
		selected: $selectedId === mod.id,
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined,
		suspend: Boolean(mod.suspend),
		sleep: Boolean(mod.sleep)
	}

	function onDelete(event: CustomEvent<MouseEvent>) {
		dispatch('delete', event)
	}
</script>

{#if mod}
	<InsertModuleButton on:new={(e) => dispatch('insert', e.detail)} />
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
					<Icon data={faRepeat} scale={0.8} />
				</div>
			</FlowModuleSchemaItem>
			<div class="flex flex-row w-full">
				<div class="w-7 shrink-0 line" />
				<div class="grow my-1 min-w-0">
					<div class="w-full">
						<FlowModuleSchemaMap bind:modules={mod.value.modules} />
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
					<Icon data={faCodeBranch} scale={0.8} />
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
					<Icon data={faCodeBranch} scale={0.8} />
				</div>
			</FlowModuleSchemaItem>
			<FlowBranchAllMap bind:module={mod} />
		</li>
	{:else}
		<li>
			<FlowModuleSchemaItem
				on:click={() => select(mod.id)}
				on:delete={onDelete}
				deletable
				id={mod.id}
				{...itemProps}
				label={mod.summary ||
					(`path` in mod.value ? mod.value.path : undefined) ||
					(mod.value.type === 'rawscript' ? `Inline ${mod.value.language}` : 'To be defined')}
			>
				<div slot="icon">
					{#if mod.value.type === 'rawscript'}
						<Icon data={faCode} scale={0.8} />
					{:else if mod.value.type === 'identity'}
						<Icon data={faLongArrowDown} scale={0.8} />
					{:else if mod.value.type === 'script'}
						<Icon data={faBuilding} scale={0.8} />
					{/if}
				</div>
			</FlowModuleSchemaItem>
		</li>
	{/if}
{/if}

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, rgb(120, 120, 120) 4px 8px)
			50%/1px 100% no-repeat;
	}
</style>
