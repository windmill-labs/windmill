<script lang="ts">
	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string
</script>

<div class="bg-red flex flex-1 h-full flex-col w-full">
	<FlowCardHeader {title}>
		<slot name="header" />
	</FlowCardHeader>
	<slot />
</div>
