<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { Loader2, Play, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import type { Job } from '$lib/gen'

	interface Props {
		isRunning?: boolean
		hover?: boolean
		selected?: boolean
		individualStepTests?: boolean
		showJobStatus?: boolean
		job?: Job
		onTestFlow?: () => void
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
	}

	let {
		isRunning,
		hover,
		selected,
		individualStepTests,
		job,
		showJobStatus,
		onTestFlow,
		onCancelTestFlow,
		onOpenPreview,
		onHideJobStatus
	}: Props = $props()

	const flowPreviewJob = $derived(showJobStatus ? job : undefined)
	const wide = $derived(hover || selected || flowPreviewJob || individualStepTests)
</script>

{#if !isRunning}
	<Button
		size="sm"
		color="dark"
		btnClasses={twMerge(
			'relative p-1.5 h-[36px] transition-all duration-200',
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
		{#if wide}
			<span transition:fade={{ duration: 100 }} class="text-xs">Test flow</span>
		{/if}
	</Button>
	{#if wide && (flowPreviewJob || individualStepTests)}
		<div
			class="flex flex-row items-center shadow-sm rounded-md mt-1"
			in:fade={{ duration: 100, delay: 200 }}
			style={`width: ${wide ? '120px' : '44.5px'}`}
		>
			<Popover class="grow min-w-0">
				<button
					class={twMerge(
						'text-xs rounded-md rounded-r-none px-1.5 h-[24px] w-full bg-surface flex flex-row items-center gap-2 justify-center transition-all duration-200 ',
						'hover:bg-surface-hover text-gray-400 hover:text-primary'
					)}
					onclick={onOpenPreview}
				>
					{#if flowPreviewJob}
						<div
							class={twMerge(
								'rounded-full h-2 w-2',
								'success' in flowPreviewJob && flowPreviewJob.success
									? 'bg-green-400'
									: 'bg-red-400'
							)}
							title={'success' in flowPreviewJob && flowPreviewJob.success ? 'Success' : 'Failed'}
						>
						</div>
					{/if}
					<span class="text-xs truncate" dir="rtl">
						{!individualStepTests && flowPreviewJob ? flowPreviewJob.id.slice(-5) : '~'}
					</span>
				</button>
				{#snippet text()}
					{#if !individualStepTests}
						See run details
					{:else}
						Open preview
					{/if}
				{/snippet}
			</Popover>
			<button
				class="h-[24px] px-1.5 bg-surface rounded-md rounded-l-none text-gray-400 hover:bg-red-500 hover:text-white"
				title="Hide jobs status"
				onclick={onHideJobStatus}
			>
				<X size={14} />
			</button>
		</div>
	{/if}
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
