<script lang="ts">
	import type { FlowModuleSchema } from '$lib/components/flows/flowState'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import {
		faClock,
		faFire,
		faFlagCheckered,
		faHourglass,
		faPen,
		faPenAlt,
		faPenRuler,
		faSliders,
		faToggleOff
	} from '@fortawesome/free-solid-svg-icons'
	import { emptyFlowModuleSchema } from '$lib/components/flows/flowStateUtils'
	import { classNames } from '$lib/utils'

	export let flowModuleSchemas: FlowModuleSchema[]
	export let prefix: string | undefined = undefined

	function insertAtIndex(index: number): void {
		flowModuleSchemas.splice(index, 0, emptyFlowModuleSchema())

		flowModuleSchemas = flowModuleSchemas
	}

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<ul class="w-full">
	{#if prefix === undefined}
		<FlowModuleSchemaItem
			hasLine={false}
			on:click={() => select('settings')}
			isFirst
			selected={$selectedId === 'settings'}
		>
			<div slot="icon">
				<Icon data={faSliders} scale={0.8} />
			</div>
			<div slot="content">
				<span>Settings</span>
			</div>
		</FlowModuleSchemaItem>

		<FlowModuleSchemaItem
			hasLine={false}
			on:click={() => select('schedules')}
			isFirst
			selected={$selectedId === 'schedules'}
		>
			<div slot="icon">
				<Icon data={faHourglass} scale={0.8} />
			</div>
			<div slot="content">
				<span>Schedules</span>
			</div>
		</FlowModuleSchemaItem>
		<div class="border-dashed border-b border-gray-400 mb-2" />
		<FlowModuleSchemaItem
			hasLine={true}
			on:click={() => select('inputs')}
			isFirst
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
		{#if flowModuleSchema.flowModule.value.type === 'forloopflow'}
			<li>
				<FlowModuleSchemaItem isFirst={true}>
					<div slot="icon">
						<span>{index}</span>
					</div>
					<div slot="content">
						<span>Loop</span>
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

				<FlowModuleSchemaItem isLast={true} color="orange">
					<div slot="icon">
						<Icon data={faFlagCheckered} scale={0.9} />
					</div>
					<div slot="content">
						<span>End of loop</span>
					</div>
				</FlowModuleSchemaItem>
			</li>
		{:else}
			<li>
				<button on:click={() => insertAtIndex(index)}>
					<div
						class={classNames(
							'flex items-center justify-center w-6 h-6 border rounded-full text-xs font-bold m-1',
							'bg-gray-50 text-gray-500'
						)}
					>
						+
					</div>
				</button>
				<FlowModuleSchemaItem
					on:click={() => select([prefix, String(index)].filter(Boolean).join('-'))}
					color={prefix ? 'orange' : 'blue'}
					isFirst={index === 0}
					isLast={index === flowModuleSchemas.length - 1}
					selected={$selectedId === [prefix, String(index)].filter(Boolean).join('-')}
				>
					<div slot="icon">
						<span>{index}</span>
					</div>
					<div slot="content">
						<span
							>{flowModuleSchema.flowModule.value.path ||
								flowModuleSchema.flowModule.summary ||
								(flowModuleSchema.flowModule.value.type === 'rawscript'
									? `Inline ${flowModuleSchema.flowModule.value.language}`
									: 'Select a script')}</span
						>
					</div>
				</FlowModuleSchemaItem>
			</li>
		{/if}
	{/each}
	<button on:click={() => insertAtIndex(flowModuleSchemas.length)}>
		<div
			class={classNames(
				'flex items-center justify-center w-6 h-6 border rounded-full text-xs font-bold m-1',
				'bg-gray-50 text-gray-500'
			)}
		>
			+
		</div>
	</button>
</ul>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
