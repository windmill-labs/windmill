<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import {
		faBug,
		faFlagCheckered,
		faPen,
		faPlus,
		faSliders,
		faSquareRootVariable
	} from '@fortawesome/free-solid-svg-icons'
	import { emptyFlowModuleSchema } from '$lib/components/flows/flowStateUtils'
	import { classNames } from '$lib/utils'
	import type { FlowModuleSchema } from '../flowState'

	export let flowModuleSchemas: FlowModuleSchema[]
	export let prefix: string | undefined = undefined

	const { select, selectedId, path } = getContext<FlowEditorContext>('FlowEditorContext')

	function insertAtIndex(index: number): void {
		flowModuleSchemas.splice(index, 0, emptyFlowModuleSchema())

		flowModuleSchemas = flowModuleSchemas
		select([prefix, String(index)].filter(Boolean).join('-'))
	}

	function removeAtIndex(index: number): void {
		flowModuleSchemas.splice(index, 1)
		flowModuleSchemas = flowModuleSchemas
	}
</script>

<ul class="w-full">
	{#if prefix === undefined}
		<div
			on:click={() => select('settings')}
			class={classNames(
				'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
				$selectedId.includes('settings')
					? 'outline outline-offset-1 outline-2  outline-slate-900'
					: ''
			)}
		>
			<Icon data={faSliders} class="mr-2" />
			<span class="font-bold">Settings</span>
		</div>

		<FlowModuleSchemaItem
			on:click={() => select('inputs')}
			isFirst
			hasLine
			selected={$selectedId === 'inputs'}
		>
			<div slot="icon">
				<Icon data={faPen} scale={0.8} />
			</div>
			<div slot="content">
				<span>Inputs</span>
			</div>
		</FlowModuleSchemaItem>
	{/if}

	{#each flowModuleSchemas as flowModuleSchema, index (index)}
		<button
			on:click={() => insertAtIndex(index)}
			type="button"
			class="text-gray-900 m-0.5 my-0.5 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<Icon data={faPlus} scale={0.8} />
		</button>
		{#if flowModuleSchema.flowModule.value.type === 'forloopflow'}
			<li>
				<FlowModuleSchemaItem
					deletable
					on:delete={() => removeAtIndex(index)}
					on:click={() => select(['loop', String(index)].join('-'))}
					selected={$selectedId === ['loop', String(index)].join('-')}
				>
					<div slot="icon">
						<span>{index}</span>
					</div>
					<div slot="content">
						<span>For loop</span>
					</div>
				</FlowModuleSchemaItem>

				<div class="flex text-xs">
					<div class="line mr-2 w-8" />

					<div class="w-full mb-2">
						<svelte:self
							prefix={String(index)}
							flowModuleSchemas={flowModuleSchema.childFlowModules}
						/>
					</div>
				</div>

				<div class="italic text-xs pl-10 text-gray-600">
					The loop's result is the list of every iteration's result.
				</div>
			</li>
		{:else}
			<li>
				<FlowModuleSchemaItem
					on:click={() => select([prefix, String(index)].filter(Boolean).join('-'))}
					color={prefix ? 'orange' : 'blue'}
					isFirst={index === 0}
					isLast={index === flowModuleSchemas.length - 1}
					selected={$selectedId === [prefix, String(index)].filter(Boolean).join('-')}
					deletable
					on:delete={() => removeAtIndex(index)}
				>
					<div slot="icon">
						<span>{index}</span>
					</div>
					<div slot="content" class="w-full">
						<input
							bind:value={flowModuleSchema.flowModule.summary}
							placeholder={flowModuleSchema.flowModule.summary ||
								flowModuleSchema.flowModule.value.path ||
								(flowModuleSchema.flowModule.value.type === 'rawscript'
									? `Inline ${flowModuleSchema.flowModule.value.language}`
									: 'Select a script')}
						/>
					</div>
				</FlowModuleSchemaItem>
			</li>
		{/if}
	{/each}

	<button
		on:click={() => insertAtIndex(flowModuleSchemas.length)}
		type="button"
		class="text-gray-900 bg-white border m-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Icon data={faPlus} scale={0.8} />
	</button>
	{#if prefix === undefined}
		<div
			on:click={() => select('failure-module')}
			class={classNames(
				'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mt-4',
				$selectedId.includes('failure-module')
					? 'outline outline-offset-1 outline-2  outline-slate-900'
					: ''
			)}
		>
			<Icon data={faBug} class="mr-2" />
			<span class="font-bold">Failure module</span>
		</div>
	{/if}
</ul>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
