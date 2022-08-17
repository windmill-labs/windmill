<script lang="ts">
	import FlowBoxHeader from './FlowBoxHeader.svelte'
	export let title: string | undefined = undefined
	let slots = $$props.$$slots
</script>

<div class="bg-white border border-gray-300 rounded-md shadow-md ">
	<FlowBoxHeader {title}>
		<slot name="header" />
	</FlowBoxHeader>
	{#if slots.content}
		<div class="p-6 ">
			<slot name="content" />
		</div>
	{/if}
</div>
