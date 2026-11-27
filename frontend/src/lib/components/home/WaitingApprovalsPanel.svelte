<script lang="ts">
	import { Button } from '$lib/components/common'
	import { PanelRightClose } from 'lucide-svelte'
	import WaitingApprovalsList from './WaitingApprovalsList.svelte'
	import type { WaitingApprovalsState } from './waitingApprovals.svelte'

	interface Props {
		approvals: WaitingApprovalsState
		onCollapse: () => void
	}

	let { approvals, onCollapse }: Props = $props()
</script>

<div class="flex flex-col min-h-0 h-full">
	<div class="flex items-center gap-2 shrink-0 pb-2">
		<h2 class="flex items-baseline gap-1.5 min-w-0 flex-1 text-[13px] font-medium text-primary">
			<span class="whitespace-nowrap">Awaiting your approval</span>
			<span class="text-xs font-normal text-hint tabular-nums">{approvals.total}</span>
		</h2>
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: PanelRightClose }}
			onclick={onCollapse}
			aria-label="Hide approvals"
			title="Hide approvals"
		/>
	</div>
	<WaitingApprovalsList {approvals} class="flex-1 min-h-0 -mx-2.5" />
</div>
