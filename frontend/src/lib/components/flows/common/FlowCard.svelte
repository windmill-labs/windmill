<script lang="ts">
	import type { FlowModule } from '$lib/gen'

	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string | undefined = undefined
	export let flowModule: FlowModule | undefined = undefined
</script>

<FlowCardHeader {title} bind:flowModule>
	<slot name="header" />
</FlowCardHeader>
<slot />
