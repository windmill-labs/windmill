<script lang="ts" module>
	export type RunsSelectionMode = 'cancel' | 're-run'
</script>

<script lang="ts">
	import { userStore, superadmin } from '$lib/stores'
	import { X, Check, ChevronDown, Loader2, SquareMousePointer } from 'lucide-svelte'
	import { Button } from '../common'
	import DropdownV2 from '../DropdownV2.svelte'

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
			size="xs"
			color="gray"
			variant="contained"
			on:click={() => onSetSelectionMode(false)}
		/>
		{#if selectionMode == 'cancel'}
			<Button
				disabled={selectionCount == 0}
				startIcon={{ icon: Check }}
				size="xs"
				color="red"
				variant="contained"
				on:click={onCancelSelectedJobs}
			>
				Cancel {jobCountString(selectionCount)}
			</Button>
		{/if}
		{#if selectionMode == 're-run'}
			<Button
				disabled={selectionCount == 0}
				startIcon={{ icon: Check }}
				size="xs"
				color="green"
				variant="contained"
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
			<div
				class="px-2 h-[30px] border flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md gap-2"
			>
				<SquareMousePointer size={16} />
				{#if !small}
					<span class="text-xs min-w-[5rem]">Batch actions</span>
				{/if}
				<ChevronDown size={16} />
			</div>
		{/snippet}
	</DropdownV2>
{/if}
