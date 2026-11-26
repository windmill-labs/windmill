<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { ChevronDown, Hourglass, PanelRightOpen } from 'lucide-svelte'
	import WaitingApprovalsList from './WaitingApprovalsList.svelte'
	import type { WaitingApprovalsState } from './waitingApprovals.svelte'

	interface Props {
		approvals: WaitingApprovalsState
		/** Absent below the width the side column needs: there is no section to add. */
		onShowSection?: () => void
		/** A narrow bar: the hourglass and the count. */
		compact?: boolean
	}

	let { approvals, onShowSection, compact = false }: Props = $props()

	let isOpen = $state(false)
	let label = $derived(
		`${approvals.total} run${approvals.total === 1 ? '' : 's'} waiting for your approval`
	)
</script>

<!-- Shaped like the recent-runs status beside it: both are statuses of the workspace's runs, where
     the buttons before them are its changes. -->
<div class="flex items-center h-[22px] pl-2.5 ml-1.5 border-l border-border-light wm-page-in">
	<Popover
		openOnHover
		debounceDelay={140}
		placement="bottom-end"
		bind:isOpen
		triggerAttrs={{ 'aria-label': label }}
		class="flex items-center gap-1.5 h-7 pl-2 pr-1.5 -mr-1.5 rounded-md text-xs font-normal text-violet-600 dark:text-violet-400 whitespace-nowrap tabular-nums hover:bg-surface-secondary {isOpen
			? 'bg-surface-secondary'
			: ''}"
	>
		{#snippet trigger()}
			<Hourglass size={14} />
			{approvals.total}{compact ? '' : ' to approve'}
			{#if !compact}
				<ChevronDown size={12} class="text-hint" />
			{/if}
		{/snippet}
		{#snippet content()}
			<div class="w-[380px] max-w-[calc(100vw-32px)] p-1.5 text-xs">
				<div class="px-2.5 pt-2 pb-1.5">
					<b class="font-semibold text-emphasis text-[12.5px]">Awaiting your approval</b>
				</div>
				<WaitingApprovalsList {approvals} class="max-h-[352px]" onOpen={() => (isOpen = false)} />
				{#if onShowSection}
					<div class="flex justify-end mt-1.5 pt-2 px-1.5 pb-1 border-t border-border-light">
						<Button
							variant="subtle"
							unifiedSize="sm"
							startIcon={{ icon: PanelRightOpen }}
							onclick={() => {
								isOpen = false
								onShowSection()
							}}
						>
							Show approvals on home
						</Button>
					</div>
				{/if}
			</div>
		{/snippet}
	</Popover>
</div>
