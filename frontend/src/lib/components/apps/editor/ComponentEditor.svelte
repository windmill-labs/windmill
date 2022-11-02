<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'

	export let component: AppComponent
	export let selected: boolean

	const { staticOutputs, selection, schemas } = getContext<AppEditorContext>('AppEditorContext')

	function removeComponent() {}
</script>

{#if component}
	<div class="h-full flex flex-col w-full">
		<span
			class={classNames(
				' text-white px-2 text-xs py-1 font-bold rounded-t-sm w-fit',
				selected ? 'bg-indigo-500' : 'bg-gray-500'
			)}
		>
			{component.type}
		</span>
		<div
			class={classNames(
				'p-2 border cursor-pointer hover:bg-blue-100 h-full bg-white',
				selected ? 'border-indigo-400' : 'border-gray-400'
			)}
		>
			{#if component.type === 'runformcomponent'}
				<RunFormComponent
					{...component}
					bind:staticOutputs={$staticOutputs[component.id]}
					bind:schema={$schemas[component.id]}
				/>
			{:else if component.type === 'displaycomponent'}
				<DisplayComponent
					{...component}
					bind:staticOutputs={$staticOutputs[component.id]}
					bind:schema={$schemas[component.id]}
				/>
			{/if}
		</div>
	</div>
{/if}
