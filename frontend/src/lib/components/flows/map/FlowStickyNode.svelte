<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'

	interface Props {
		disableSettings?: boolean
		disableStaticInputs?: boolean
		smallErrorHandler: boolean
	}

	let { disableSettings, disableStaticInputs, smallErrorHandler }: Props = $props()

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const nodeClass =
		'border w-fit rounded p-2 bg-surface text-sm cursor-pointer flex items-center h-[32px] hover:!bg-surface-secondary active:!bg-surface'
	const nodeSelectedClass =
		'outline outline-offset-1 outline-2 outline-slate-800 dark:bg-white/5 dark:outline-slate-400/60 dark:outline-gray-400'
</script>

<div class="flex flex-row gap-2 p-1.5 rounded shadow-md bg-surface">
	{#if !disableSettings}
		<button
			onclick={() => ($selectedId = 'settings-metadata')}
			class={twMerge(nodeClass, $selectedId?.startsWith('settings') ? nodeSelectedClass : '')}
			title="Settings"
		>
			<Settings size={16} />
			{#if flowStore.val.value.same_worker}
				<span class="ml-2 h-[18px] flex items-center">
					<Badge color="blue" baseClass="truncate">./shared</Badge>
				</span>
			{/if}
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

	<FlowErrorHandlerItem small={smallErrorHandler} on:generateStep />
</div>
