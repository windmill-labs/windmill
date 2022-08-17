<script lang="ts">
	import FlowBoxHeader from './FlowBoxHeader.svelte'
	export let title: string | undefined = undefined

	export let opened: boolean = false

	const borderColor = opened ? 'border-blue-700' : 'border-gray-300'

	let slots = $$props.$$slots
</script>

<div class={`bg-white border ${borderColor} rounded-md`}>
	<FlowBoxHeader {title}>
		<slot name="header" />
	</FlowBoxHeader>
	{#if slots.content}
		<slot name="content" />
	{/if}
</div>
