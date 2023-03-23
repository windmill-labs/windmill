<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Maximize, Minimize } from 'lucide-svelte'

	export let expanded: boolean = false
</script>

<Popover notClickable placement="bottom" disapperTimoout={0} class="px-1">
	<Button
		on:click={() => {
			expanded = !expanded
		}}
		color="dark"
		size="xs"
		variant="contained"
		btnClasses="px-2 !h-[31px]"
	>
		{#if !expanded}
			<div class="flex flex-row gap-2 items-center">
				<Maximize size="14" />
			</div>
		{:else}
			<div class="flex flex-row gap-2 items-center">
				<Minimize size="14" />
			</div>
		{/if}
	</Button>
	<svelte:fragment slot="text">
		{expanded ? 'Minimize' : 'Maximize'}
	</svelte:fragment>
</Popover>
