<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'

	export let component: AppComponent

	const { staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function removeComponent() {}
</script>

{#if component === undefined}
	<div class="h-full flex flex-col">
		<span class="bg-gray-500 text-white px-2 text-xs py-1 font-bold rounded-t-sm w-fit">
			Empty component
		</span>
		<div
			class="p-2 border-dashed border border-gray-400  cursor-pointer hover:bg-blue-100 h-full flex justify-center items-center text-sm"
		>
			Empty component
		</div>
	</div>
{:else}
	<div class="h-full flex flex-col">
		<span class="bg-gray-500 text-white px-2 text-xs py-1 font-bold rounded-t-sm w-fit">
			{component.type}
		</span>
		<div class="p-2 border border-gray-400  cursor-pointer hover:bg-blue-100 h-full">
			{#if component.type === 'runformcomponent'}
				<RunFormComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'displaycomponent'}
				<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{/if}
		</div>
	</div>
{/if}
