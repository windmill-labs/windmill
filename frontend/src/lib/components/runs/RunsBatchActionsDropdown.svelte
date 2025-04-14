<script lang="ts" context="module">
	export type RunsSelectionMode = 'cancel' | 're-run'
</script>

<script lang="ts">
	import { userStore, superadmin } from '$lib/stores'
	import { X, Check, ChevronDown, Loader2 } from 'lucide-svelte'
	import { Button } from '../common'
	import DropdownV2 from '../DropdownV2.svelte'

	export let isLoading = false
	export let selectionCount: number
	export let selectionMode: RunsSelectionMode | false
	export let onSetSelectionMode: (mode: RunsSelectionMode | false) => void
	export let onCancelSelectedJobs: () => void
	export let onCancelFilteredJobs: () => void
	export let onReRunSelectedJobs: () => void
	export let onReRunFilteredJobs: () => void

	function jobCountString(count: number) {
		return `${count} ${count == 1 ? 'job' : 'jobs'}`
	}
</script>

{#if isLoading}
	<Button size="xs" color="light" disabled>
		<Loader2 class="animate-spin" size={20} />
	</Button>
{:else if selectionMode}
	<div class="mt-1 p-2 h-8 flex flex-row items-center gap-1">
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
		class="w-fit mx-auto"
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
		<svelte:fragment slot="buttonReplacement">
			<div
				class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
			>
				<span class="text-xs min-w-[5rem]">Batch actions</span>
				<ChevronDown class="w-5 h-5" />
			</div>
		</svelte:fragment>
	</DropdownV2>
{/if}
