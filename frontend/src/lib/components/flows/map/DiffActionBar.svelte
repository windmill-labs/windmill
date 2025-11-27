<script lang="ts">
	import { DiffIcon } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import type { ModuleActionInfo } from '$lib/components/copilot/chat/flow/core'
	import type { FlowDiffManager } from '../flowDiffManager.svelte'
	import type { StateStore } from '$lib/utils'
	import type { OpenFlow } from '$lib/gen'

	interface Props {
		moduleId: string
		moduleAction: ModuleActionInfo | undefined
		diffManager: FlowDiffManager | undefined
		flowStore: StateStore<OpenFlow> | undefined
		placement?: 'top' | 'bottom'
	}

	let { moduleId, moduleAction, diffManager, flowStore, placement = 'top' }: Props = $props()
</script>

{#if moduleAction && diffManager}
	<div
		class="absolute right-0 left-0 {placement === 'top'
			? 'top-0 -translate-y-full'
			: 'bottom-0 translate-y-full'} flex justify-start gap-1 z-50"
	>
		<!-- Diff button shows if action is 'modified' -->
		{#if moduleAction.action === 'modified' && diffManager.beforeFlow}
			<Button
				class="p-1 bg-surface hover:bg-surface-hover rounded-t-md text-3xs font-normal flex flex-row items-center gap-1 text-orange-800 dark:text-orange-400"
				onClick={() => {
					diffManager?.showModuleDiff(moduleId)
				}}
				startIcon={{ icon: DiffIcon }}
			>
				Diff
			</Button>
		{/if}
		<!-- Accept/Reject buttons show if pending (editMode was true when diff computed) -->
		{#if moduleAction.pending}
			<Button
				size="xs"
				color="green"
				class="p-1 bg-surface hover:bg-surface-hover rounded-t-md text-3xs font-normal flex flex-row items-center gap-1"
				onClick={() => {
					if (flowStore) diffManager?.acceptModule(moduleId, flowStore)
				}}
			>
				✓ Accept
			</Button>
			<Button
				size="xs"
				color="red"
				class="p-1 bg-surface hover:bg-surface-hover rounded-t-md text-3xs font-normal flex flex-row items-center gap-1"
				onClick={() => {
					if (flowStore) diffManager?.rejectModule(moduleId, flowStore)
				}}
			>
				✗ Reject
			</Button>
		{/if}
	</div>
{/if}
