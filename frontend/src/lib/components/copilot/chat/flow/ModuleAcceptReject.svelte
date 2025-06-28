<script module>
	export const getAiModuleAction = (id: string | undefined) => {
		if (!id) return undefined
		return aiChatManager.flowAiChatHelpers?.getModuleAction(id)
	}

	export function aiModuleActionToBgColor(action: AIModuleAction | undefined) {
		switch (action) {
			case 'modified':
				return '!bg-orange-200 dark:!bg-orange-800'
			case 'added':
				return '!bg-green-200 dark:!bg-green-800'
			case 'removed':
				return '!bg-red-200/50 dark:!bg-red-800/50'
			default:
				return ''
		}
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { Check, DiffIcon, X } from 'lucide-svelte'
	import { aiChatManager } from '../AIChatManager.svelte'
	import type { AIModuleAction } from './core'

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
