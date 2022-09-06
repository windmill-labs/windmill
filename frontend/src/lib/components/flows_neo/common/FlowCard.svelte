<script lang="ts">
	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string
</script>

<div class="bg-white">
	<FlowCardHeader {title}>
		<slot name="header" />
	</FlowCardHeader>
	<div class="p-4">
		<slot />
	</div>
</div>
