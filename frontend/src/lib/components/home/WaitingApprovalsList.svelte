<script lang="ts">
	import { Hourglass } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		approvalAge,
		approvalDetail,
		approvalHref,
		approvalTitle,
		type WaitingApprovalsState
	} from './waitingApprovals.svelte'

	interface Props {
		approvals: WaitingApprovalsState
		/** Caps the list's height; it scrolls inside it, so a long queue never pushes past it. */
		class?: string
		onOpen?: () => void
	}

	let { approvals, class: classNames = '', onOpen }: Props = $props()

	let hidden = $derived(approvals.total - approvals.approvals.length)
</script>

<div class="overflow-y-auto overscroll-contain scrollbar-subtle {classNames}">
	{#each approvals.approvals as a (a.job_id)}
		<a
			href={approvalHref(a, $workspaceStore)}
			target="_blank"
			rel="noopener"
			onclick={onOpen}
			class="flex items-center gap-2.5 min-h-[42px] px-2.5 py-[5px] rounded-md hover:bg-surface-hover focus-visible:bg-surface-hover outline-none"
		>
			<span class="flex w-3.5 shrink-0 justify-center text-violet-500">
				<Hourglass size={14} />
			</span>
			<span class="flex flex-col min-w-0 flex-1">
				<span class="truncate text-[12.5px] font-medium text-primary">{approvalTitle(a)}</span>
				<span class="truncate text-[10.5px] text-hint">{approvalDetail(a)}</span>
			</span>
			<span class="w-[46px] shrink-0 text-right text-[11px] text-hint">
				{approvalAge(a, approvals.now)}
			</span>
		</a>
	{/each}
</div>
{#if hidden > 0}
	<!-- The server lists the most recent only: say what is left out rather than end as if complete. -->
	<p class="px-2.5 pt-1 text-[11.5px] text-hint">
		{hidden} older run{hidden === 1 ? '' : 's'} also waiting
	</p>
{/if}
