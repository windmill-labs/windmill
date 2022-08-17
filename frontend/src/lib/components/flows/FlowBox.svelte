<script lang="ts">
	import FlowBoxHeader from './FlowBoxHeader.svelte'
	export let title: string | undefined = undefined
	let slots = $$props.$$slots
</script>

<div class="bg-white border border-gray-300 rounded-md">
	<FlowBoxHeader {title}>
		<slot name="header" />
	</FlowBoxHeader>
	{#if slots.content}
		<slot name="content" />
	{/if}
</div>
