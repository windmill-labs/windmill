<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'
	import RunnableJobPanel from './RunnableJobPanel.svelte'

	export let rightPanelSize: number = 0
	export let centerPanelWidth: number = 0
	export let runnablePanelSize: number = 0
</script>

{#if rightPanelSize !== 0}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div class={twMerge('relative h-full w-full overflow-x-visible')} on:mouseenter on:mouseleave>
		<InlineScriptsPanel on:hidePanel />
		<RunnableJobPanel hidden={runnablePanelSize === 0} />
		<slot />
	</div>
{:else}
	<div class="flex flex-row relative w-full h-full">
		<InlineScriptsPanel width={centerPanelWidth - 400} on:hidePanel />

		<slot />
	</div>
{/if}
