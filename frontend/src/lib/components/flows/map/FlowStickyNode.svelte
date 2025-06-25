<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		disableSettings?: boolean
		disableStaticInputs?: boolean
	}

	let { disableSettings, disableStaticInputs }: Props = $props()

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const nodeClass =
		'border w-fit rounded p-2 bg-surface text-sm cursor-pointer flex items-center h-[32px] hover:!bg-surface-secondary active:!bg-surface'
	const nodeSelectedClass =
		'outline outline-offset-1 outline-2 outline-slate-800 dark:bg-white/5 dark:outline-slate-400/60 dark:outline-gray-400'
</script>

<div class="flex flex-row gap-2">
	{#if !disableSettings}
		<button
			onclick={() => ($selectedId = 'settings-metadata')}
			class={twMerge(nodeClass, $selectedId?.startsWith('settings') ? nodeSelectedClass : '')}
			title="Settings"
		>
			<Settings size={16} />
			<span
				class="text-xs font-bold flex flex-row justify-between w-fit gap-2 items-center truncate ml-1"
				title="Settings"
			>
				<span class="h-[18px] flex items-center">
					{#if flowStore.val.value.same_worker}
						<Badge color="blue" baseClass="truncate">./shared</Badge>
					{/if}
				</span>
			</span>
		</button>
	{/if}
	{#if !disableStaticInputs}
		<button
			onclick={() => ($selectedId = 'constants')}
			class={twMerge(nodeClass, $selectedId == 'constants' ? nodeSelectedClass : '')}
			title="All Static Inputs"
		>
			<DollarSign size={16} />
		</button>
	{/if}
</div>
