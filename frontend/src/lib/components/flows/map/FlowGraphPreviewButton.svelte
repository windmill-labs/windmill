<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { Loader2, Play } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		isRunning?: boolean
		hover?: boolean
		selected?: boolean
		onTestFlow?: () => void
		onCancelTestFlow?: () => void
	}

	let { isRunning, hover, selected, onTestFlow, onCancelTestFlow }: Props = $props()
</script>

{#if !isRunning}
	<Button
		size="sm"
		color="dark"
		title="Run"
		btnClasses={twMerge(
			'p-1.5 h-[34px] transition-all duration-200',
			hover || selected ? 'w-[120px]' : 'w-[44.5px]'
		)}
		on:click={() => {
			onTestFlow?.()
		}}
	>
		{#if isRunning}
			<Loader2 size={16} class="animate-spin" />
		{:else}
			<Play size={16} />
		{/if}
		{#if hover || selected}
			<span transition:fade={{ duration: 100 }} class="text-xs">Test flow</span>
		{/if}
	</Button>
{:else}
	<Button
		size="xs"
		color="red"
		variant="contained"
		btnClasses="h-[34px] w-[120px] p-1.5"
		on:click={async () => {
			onCancelTestFlow?.()
		}}
	>
		<Loader2 size={16} class="animate-spin" />
		<span transition:fade={{ duration: 100 }} class="text-xs">Cancel</span>
	</Button>
{/if}
