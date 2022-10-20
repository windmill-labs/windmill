<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'

	export let mod: FlowModule
	export let index: number
	export let color: 'blue' | 'orange' | 'indigo' = 'blue'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()
</script>

{#if mod}
	<button
		on:click={() => dispatch('insert')}
		type="button"
		class="text-gray-900 m-0.5 my-0.5 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Icon data={faPlus} scale={0.8} />
	</button>
	{#if mod.value.type === 'forloopflow'}
		<li>
			<FlowModuleSchemaItem
				deletable
				on:delete
				on:click={() => select(mod.id)}
				selected={$selectedId === mod.id}
				retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
				earlyStop={mod.stop_after_if != undefined}
				suspend={Boolean(mod.suspend)}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="truncate block w-full">
					<span>{mod.summary || 'For loop'}</span>
				</div>
			</FlowModuleSchemaItem>

			<div class="flex text-xs">
				<div class="line mr-2" />

				<div class="w-full my-2">
					<FlowModuleSchemaMap modules={mod.value.modules} color="orange" />
				</div>
			</div>
		</li>
	{:else}
		<li>
			<FlowModuleSchemaItem
				on:click={() => select(mod.id)}
				on:delete
				{color}
				selected={$selectedId === mod.id}
				deletable
				retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
				earlyStop={mod.stop_after_if != undefined}
				suspend={Boolean(mod.suspend)}
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
