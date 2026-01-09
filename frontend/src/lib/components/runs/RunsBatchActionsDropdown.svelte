<script lang="ts">
	import { userStore, superadmin } from '$lib/stores'
	import { X, Check, ChevronDown, Loader2, SquareMousePointer } from 'lucide-svelte'
	import { Button } from '../common'
	import DropdownV2 from '../DropdownV2.svelte'
	import type { RunsSelectionMode } from '$lib/utils'

	interface Props {
		isLoading?: boolean
		selectionCount: number
		selectionMode: RunsSelectionMode | false
		small?: boolean
		onSetSelectionMode: (mode: RunsSelectionMode | false) => void
		onCancelSelectedJobs: () => void
		onCancelFilteredJobs: () => void
		onReRunSelectedJobs: () => void
		onReRunFilteredJobs: () => void
	}

	let {
		isLoading = false,
		selectionCount,
		selectionMode,
		small = false,
		onSetSelectionMode,
		onCancelSelectedJobs,
		onCancelFilteredJobs,
		onReRunSelectedJobs,
		onReRunFilteredJobs
	}: Props = $props()

	function jobCountString(count: number) {
		return `${count} ${count == 1 ? 'job' : 'jobs'}`
	}
</script>

{#if isLoading}
	<Button size="xs" color="light" disabled>
		<Loader2 class="animate-spin" size={20} />
	</Button>
{:else if selectionMode}
	<div class="h-8 flex flex-row items-center gap-1">
		<Button
			startIcon={{ icon: X }}
			iconOnly
			unifiedSize="md"
			variant="default"
			on:click={() => onSetSelectionMode(false)}
		/>
		{#if selectionMode == 'cancel'}
			<Button
				disabled={selectionCount == 0}
				startIcon={{ icon: Check }}
				unifiedSize="md"
				variant="accent"
				destructive
				on:click={onCancelSelectedJobs}
			>
				Cancel {jobCountString(selectionCount)}
			</Button>
		{/if}
		{#if selectionMode == 're-run'}
			<Button
				disabled={selectionCount == 0}
				startIcon={{ icon: Check }}
				unifiedSize="md"
				variant="accent"
				on:click={onReRunSelectedJobs}
			>
				Re-run {jobCountString(selectionCount)}
			</Button>
		{/if}
	</div>
{:else}
	<DropdownV2
		class="w-fit"
		items={[
			{
				displayName: 'Select jobs to cancel',
				action: () => onSetSelectionMode('cancel')
			},
			...($userStore?.is_admin || $superadmin
				? [{ displayName: 'Cancel all jobs matching filters', action: onCancelFilteredJobs }]
				: []),
			{
				displayName: 'Select jobs to re-run',
				action: () => onSetSelectionMode('re-run')
			},
			...($userStore?.is_admin || $superadmin
				? [{ displayName: 'Re-run all jobs matching filters', action: onReRunFilteredJobs }]
				: [])
		]}
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				variant="default"
				unifiedSize="md"
				startIcon={{ icon: SquareMousePointer }}
				endIcon={{ icon: ChevronDown }}
			>
				{#if !small}
					<span>Batch actions</span>
				{/if}
			</Button>
		{/snippet}
	</DropdownV2>
{/if}
