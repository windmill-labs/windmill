<script lang="ts">
	import type { FlowModule } from '$lib/gen'

	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string | undefined = undefined
	export let flowModule: FlowModule | undefined = undefined
	export let headerHeight: number = 0
</script>

<FlowCardHeader {title} bind:flowModule bind:height={headerHeight}>
	<slot name="header" />
</FlowCardHeader>
<slot />
