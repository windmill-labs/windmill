<script lang="ts">
	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string
</script>

<FlowCardHeader {title}>
	<slot name="header" />
</FlowCardHeader>
<slot />
