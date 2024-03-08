<script lang="ts">
	import type { FlowModule } from '$lib/gen'

	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string | undefined = undefined
	export let flowModule: FlowModule | undefined = undefined
	export let noEditor: boolean
</script>

{#if !noEditor}
	<FlowCardHeader on:reload {title} bind:flowModule>
		<slot name="header" />
	</FlowCardHeader>
{/if}

<slot />
