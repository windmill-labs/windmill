<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { Loader2, Play } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import type { previewContext } from '../utils'
	import type { Job } from '$lib/gen'

	interface Props {
		isRunning?: boolean
		hover?: boolean
		selected?: boolean
		onTestFlow?: () => void
		onCancelTestFlow?: () => void
		onOpenDetails?: () => void
	}

	let { isRunning, hover, selected, onTestFlow, onCancelTestFlow, onOpenDetails }: Props = $props()

	const jobContext = getContext<previewContext>('previewContext')
	const job = $derived(jobContext?.getJob())

	$inspect('dbg job', job)

	const wide = $derived(hover || selected)
</script>

{#if !isRunning}
	<Button
		size="sm"
		color="dark"
		title="Run"
		btnClasses={twMerge(
			'relative p-1.5 h-[34px] transition-all duration-200',
			wide ? 'w-[120px]' : 'w-[44.5px]'
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

		{#if job && wide}
			<div class="absolute top-[38px] left-0 right-0">
				<button
					class={twMerge(
						'text-xs rounded px-1.5 h-[20px] bg-surface flex flex-row items-center gap-2 justify-center transition-all duration-200',
						'hover:bg-surface-hover',
						wide ? 'w-[120px]' : 'w-[44.5px]'
					)}
					in:fade={{ duration: 100, delay: 200 }}
					onclick={(e) => {
						e.stopPropagation()
						onOpenDetails?.()
					}}
				>
					{@render dotStatus(job, true)}
					{#if wide}
						<span class="text-2xs text-gray-400 hover:text-primary"> Open preview </span>
					{/if}
				</button>
			</div>
		{:else if job}
			<div class="absolute -top-1 -right-1" out:fade={{ duration: 100 }}>
				{@render dotStatus(job)}
			</div>
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

{#snippet dotStatus(job: Job, small: boolean = false)}
	<div
		class={twMerge(
			'rounded-full',
			small ? 'h-2 w-2' : 'h-3 w-3',
			'success' in job && job.success ? 'bg-green-400' : 'bg-red-400'
		)}
		title={'success' in job && job.success ? 'Success' : 'Failed'}
	>
	</div>
{/snippet}
