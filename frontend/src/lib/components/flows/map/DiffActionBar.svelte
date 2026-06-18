<script lang="ts">
	import { DiffIcon, Check, X } from 'lucide-svelte'
	import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'
	import type { FlowDiffManager } from '../flowDiffManager.svelte'
	import type { StateStore } from '$lib/utils'
	import type { OpenFlow } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'

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
		class={twMerge(
			'absolute right-0 left-0 flex flex-row z-20',
			placement === 'top' ? 'top-0 -translate-y-full' : 'bottom-0 translate-y-full',
			moduleAction.action === 'modified' ? 'justify-between' : 'justify-end'
		)}
	>
		{#if moduleAction?.action === 'modified' && diffManager.beforeFlow}
			<button
				class="p-1 bg-surface hover:bg-surface-hover rounded-t-md text-3xs font-normal flex flex-row items-center gap-1 text-orange-800 dark:text-orange-400"
				onclick={() => {
					diffManager?.showModuleDiff(moduleId)
				}}
			>
				<DiffIcon size={14} /> Diff
			</button>
		{/if}
		{#if moduleAction?.pending}
			<div
				class={twMerge(
					'flex flex-row bg-surface overflow-hidden',
					placement === 'top' ? 'rounded-t-md' : 'rounded-b-md'
				)}
			>
				<button
					class="p-1 bg-green-500 text-white hover:bg-green-600 text-3xs font-normal flex flex-row items-center gap-1"
					onclick={() => {
						if (flowStore) diffManager?.acceptModule(moduleId, flowStore)
					}}
				>
					<Check size={14} /> Accept
				</button>
				<button
					class="p-1 hover:bg-red-500 hover:text-white text-3xs font-normal flex flex-row items-center gap-1"
					onclick={() => {
						if (flowStore) diffManager?.rejectModule(moduleId, flowStore)
					}}
				>
					<X size={14} /> Reject
				</button>
			</div>
		{/if}
	</div>
{/if}
