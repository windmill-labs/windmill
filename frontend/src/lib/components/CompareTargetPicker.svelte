<script lang="ts">
	import { Building, Target } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Select from './select/Select.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { userWorkspaces } from '$lib/stores'

	interface Props {
		/** The workspace being merged from — never a valid destination. */
		currentWorkspaceId: string
		/** The destination currently being compared against. */
		targetWorkspaceId: string
		/** The lineage destination, when this workspace has one. Offered as the way
		 * back from an arbitrary target, since only it has a continuous tally. */
		parentWorkspaceId?: string
		disabled?: boolean
		onSelected: (target: string) => void
		/** Replaces the default "Change target" button. Lets a caller that already
		 * names the destination on screen turn that into the trigger, rather than
		 * repeating it in a button beside it. */
		triggerContent?: import('svelte').Snippet
	}

	let {
		currentWorkspaceId,
		targetWorkspaceId,
		parentWorkspaceId,
		disabled = false,
		onSelected,
		triggerContent
	}: Props = $props()

	let picked = $state<string | undefined>(undefined)

	// A direct child (fork or dev workspace) is excluded: the pair is a lineage pair,
	// which the scan refuses, so it would offer a "Compute diff" that can only fail.
	// The lineage parent stays — picking it returns to the tallied comparison.
	let candidates = $derived(
		$userWorkspaces
			.filter((w) => w.id !== currentWorkspaceId && w.parent_workspace_id !== currentWorkspaceId)
			.map((w) => ({
				label:
					w.id === parentWorkspaceId
						? `${w.name} (${w.id}), parent workspace`
						: `${w.name} (${w.id})`,
				value: w.id
			}))
	)
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
	{disabled}
	closeButton
	usePointerDownOutside
	contentClasses="p-4"
>
	{#snippet trigger()}
		{#if triggerContent}
			{@render triggerContent()}
		{:else}
			<Button
				variant="subtle"
				unifiedSize="xs"
				nonCaptureEvent
				{disabled}
				startIcon={{ icon: Target }}
				title="Compare against a workspace other than {parentWorkspaceId ?? 'the default target'}"
			>
				Change target
			</Button>
		{/if}
	{/snippet}
	{#snippet content({ close })}
		<div class="flex flex-col gap-3 w-80">
			<!-- pr-5 keeps the first line clear of the popover's close button. -->
			<div class="text-xs text-secondary pr-5">
				Merging into a workspace outside this one's lineage is meant for one-off migrations.
				Windmill only tracks changes continuously between a workspace and its parent, so comparing
				against any other target computes a full diff over every item in both workspaces.
			</div>
			<Select
				items={candidates}
				bind:value={picked}
				placeholder="Select a workspace"
				clearable
				disablePortal
			/>
			<div class="flex items-center gap-2">
				<Button
					variant="accent"
					unifiedSize="xs"
					disabled={!picked || picked === targetWorkspaceId}
					onclick={() => {
						if (!picked) return
						onSelected(picked)
						close()
					}}
				>
					Compare
				</Button>
				{#if parentWorkspaceId && targetWorkspaceId !== parentWorkspaceId}
					<Button
						variant="subtle"
						unifiedSize="xs"
						startIcon={{ icon: Building }}
						onclick={() => {
							onSelected(parentWorkspaceId)
							close()
						}}
					>
						Back to {parentWorkspaceId}
					</Button>
				{/if}
			</div>
		</div>
	{/snippet}
</Popover>
