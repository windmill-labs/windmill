<script lang="ts" context="module">
	export type RunsSelectionMode = 'cancel' | 're-run'
</script>

<script lang="ts">
	import { userStore, superadmin } from '$lib/stores'
	import { X, Check, ChevronDown } from 'lucide-svelte'
	import { Button } from '../common'
	import DropdownV2 from '../DropdownV2.svelte'

	export let selectionCount: number
	export let selectionMode: RunsSelectionMode | false
	export let onSetSelectionMode: (mode: RunsSelectionMode | false) => void
	export let onCancelSelectedJobs: () => void
	export let onCancelFilteredJobs: () => void

	function jobCountString(count: number) {
		return `${count} ${count == 1 ? 'job' : 'jobs'}`
	}
</script>

{#if selectionMode}
	<div class="mt-1 p-2 h-8 flex flex-row items-center gap-1">
		<Button
			startIcon={{ icon: X }}
			size="xs"
			color="gray"
			variant="contained"
			on:click={() => onSetSelectionMode(false)}
		/>
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
	</div>
{:else if !$userStore?.is_admin && !$superadmin}
	<DropdownV2
		items={[
			{
				displayName: 'Select jobs to cancel',
				action: () => onSetSelectionMode('cancel')
			}
		]}
	>
		<svelte:fragment slot="buttonReplacement">
			<div
				class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
			>
				<span class="text-xs min-w-[5rem]">Cancel jobs</span>
				<ChevronDown class="w-5 h-5" />
			</div>
		</svelte:fragment>
	</DropdownV2>
{:else}
	<DropdownV2
		items={[
			{
				displayName: 'Select jobs to cancel',
				action: () => onSetSelectionMode('cancel')
			},
			{ displayName: 'Cancel all jobs matching filters', action: onCancelFilteredJobs }
		]}
	>
		<svelte:fragment slot="buttonReplacement">
			<div
				class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
			>
				<span class="text-xs min-w-[5rem]">Cancel jobs</span>
				<ChevronDown class="w-5 h-5" />
			</div>
		</svelte:fragment>
	</DropdownV2>
{/if}
