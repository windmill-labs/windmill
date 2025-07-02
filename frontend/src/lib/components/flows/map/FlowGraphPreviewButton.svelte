<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { Loader2, Play } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import type { previewContext } from '../utils'

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
	<div
		class={twMerge(
			'flex flex-col transition-all duration-200 h-34',
			wide ? 'w-[120px]' : 'w-[44.5px]'
		)}
	>
		<div class="grow min-h-0">
			<Button
				size="sm"
				color="dark"
				title="Run"
				wrapperClasses="h-full"
				btnClasses={twMerge('p-1.5 h-full', job ? 'rounded-b-none' : '')}
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
		</div>
		{#if job}
			{@const bgStatus = 'success' in job && job.success ? 'bg-green-400' : 'bg-red-400'}
			<button
				onclick={onOpenDetails}
				class={twMerge(
					'text-xs rounded-b-md w-full px-1.5 h-[12px] bg-surface flex flex-row items-center gap-1 justify-center'
				)}
			>
				<div
					class={twMerge('rounded-full w-2 h-2', bgStatus)}
					title={'success' in job && job.success ? 'Success' : 'Failed'}
				>
				</div>
				{#if wide}
					<span in:fade={{ duration: 100, delay: 200 }} class="text-xs2"> Open preview </span>
				{/if}
			</button>
		{/if}
	</div>
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
