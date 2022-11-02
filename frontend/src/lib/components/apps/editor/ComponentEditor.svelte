<script lang="ts">
	import { getContext } from 'svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'

	export let component: AppComponent

	const { staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function removeComponent() {}
</script>

{#if component}
	<div class="h-full flex flex-col w-full">
		<span class="bg-gray-500 text-white px-2 text-xs py-1 font-bold rounded-t-sm w-fit">
			{component.type}
		</span>
		<div class="p-2 border border-gray-400  cursor-pointer hover:bg-blue-100 h-full bg-white">
			{#if component.type === 'runformcomponent'}
				<RunFormComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'displaycomponent'}
				<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{/if}
		</div>
	</div>
{/if}
