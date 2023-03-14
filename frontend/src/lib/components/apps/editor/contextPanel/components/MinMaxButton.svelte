<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Maximize, Minimize } from 'lucide-svelte'

	export let expanded: boolean = false
</script>

<Button
	on:click={() => {
		expanded = !expanded
	}}
	color="light"
	size="xs"
	variant="border"
>
	{#if !expanded}
		<div class="flex flex-row gap-2">
			<Maximize size="14" />
			Maximize
		</div>
	{:else}
		<div class="flex flex-row gap-2">
			<Minimize size="14" />
			Minimize
		</div>
	{/if}
</Button>
