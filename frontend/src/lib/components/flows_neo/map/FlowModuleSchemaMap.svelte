<script lang="ts">
	import type { FlowModuleSchema } from '$lib/components/flows/flowState'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'

	export let flowModuleSchemas: FlowModuleSchema[]
	export let prefix: string | undefined = undefined

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<ul class="w-full">
	{#each flowModuleSchemas as flowModuleSchema, index (index)}
		{#if flowModuleSchema.flowModule.value.type === 'forloopflow'}
			<li>
				<FlowModuleSchemaItem isFirst={true}>
					<div slot="icon">
						<span>1</span>
					</div>
					<div slot="content">
						<span>Start of loop</span>
					</div>
				</FlowModuleSchemaItem>

				<div class="flex text-xs" on:click={() => select([prefix, String(index)].join('-'))}>
					<div class="line mr-2 w-8" />

					<div class="w-full mb-2">
						<svelte:self
							prefix={String(index)}
							flowModuleSchemas={flowModuleSchema.childFlowModules}
						/>
					</div>
				</div>

				<FlowModuleSchemaItem isLast={true}>
					<div slot="icon">
						<span>1</span>
					</div>
					<div slot="content">
						<span>End of loop</span>
					</div>
				</FlowModuleSchemaItem>
			</li>
		{:else}
			<li>
				<FlowModuleSchemaItem
					on:click={() => select([prefix, String(index)].join('-'))}
					color={prefix ? 'orange' : 'blue'}
					isFirst={index === 0}
					isLast={index === flowModuleSchemas.length - 1}
				>
					<div slot="icon">
						<span>{index}</span>
					</div>
					<div slot="content">
						<span>{index + 1}</span>
					</div>
				</FlowModuleSchemaItem>
			</li>
		{/if}
	{/each}
</ul>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
