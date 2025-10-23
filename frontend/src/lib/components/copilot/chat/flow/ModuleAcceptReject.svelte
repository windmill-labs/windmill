<script module lang="ts">
	import type { AIModuleAction } from './core'

	/**
	 * Check if debug diff mode is enabled via localStorage
	 * Usage: localStorage.setItem('windmill_debug_diff_mode', 'true')
	 */
	function isDebugDiffModeEnabled(): boolean {
		if (typeof window === 'undefined') return false
		return localStorage.getItem('windmill_debug_diff_mode') === 'true'
	}

	export const getAiModuleAction = (id: string | undefined): AIModuleAction | undefined => {
		if (!id) return undefined
		const realAction = aiChatManager.flowAiChatHelpers?.getModuleAction(id)

		// In debug mode, show all modules as modified if they don't have a real action
		if (isDebugDiffModeEnabled() && !realAction) {
			return 'modified'
		}

		return realAction
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { Check, DiffIcon, X } from 'lucide-svelte'
	import { aiChatManager } from '../AIChatManager.svelte'

	let {
		id,
		action,
		placement = 'top'
	}: {
		id: string | undefined
		action: AIModuleAction | undefined
		placement?: 'top' | 'bottom'
	} = $props()
</script>

{#if action && id}
	<div
		class={twMerge(
			'absolute right-0 left-0 flex flex-row ',
			placement === 'top' ? 'top-0 -translate-y-full' : 'bottom-0 translate-y-full',
			action === 'modified' ? 'justify-between' : 'justify-end'
		)}
	>
		{#if action === 'modified'}
			<button
				class="p-1 bg-surface hover:bg-surface-hover rounded-t-md text-3xs font-normal flex flex-row items-center gap-1 text-orange-800 dark:text-orange-400"
				onclick={() => {
					aiChatManager.flowAiChatHelpers?.showModuleDiff(id)
				}}
			>
				<DiffIcon size={14} /> Diff
			</button>
		{/if}
		<div
			class={twMerge(
				'flex flex-row bg-surface overflow-hidden',
				placement === 'top' ? 'rounded-t-md' : 'rounded-b-md'
			)}
		>
			<button
				class="p-1 bg-green-500 text-white hover:bg-green-600 text-3xs font-normal flex flex-row items-center gap-1"
				onclick={() => aiChatManager.flowAiChatHelpers?.acceptModuleAction(id)}
			>
				<Check size={14} /> Accept
			</button>
			<button
				class="p-1 hover:bg-red-500 hover:text-white text-3xs font-normal flex flex-row items-center gap-1"
				onclick={() => aiChatManager.flowAiChatHelpers?.revertModuleAction(id)}
			>
				<X size={14} /> Reject
			</button>
		</div>
	</div>
{/if}
