<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import { SIMPLIFIED, type SimplifiedBoxData } from './simplifiedLayout'

	let { data, selected }: { data: SimplifiedBoxData; selected?: boolean } = $props()

	let header = $derived(data.header)

	const headerTop = SIMPLIFIED.loopPad / 2
</script>

<!-- A bracket over the group rather than a box around it: only the label takes clicks, so the
     group's empty area stays part of the canvas. -->
<div class="relative pointer-events-none" style="width: {data.width}px; height: {data.height}px;">
	<div
		class={twMerge(
			'absolute inset-x-0 h-2.5 rounded-t-lg border-t border-x',
			selected ? 'border-border-selected' : 'border-gray-300 dark:border-gray-600'
		)}
		style="top: {headerTop + SIMPLIFIED.loopHeader + 1}px;"
	></div>
	<div
		class="absolute left-1 max-w-[calc(100%-8px)] inline-flex items-center gap-1.5 px-1.5 text-2xs text-secondary min-w-0 cursor-pointer pointer-events-auto"
		style="top: {headerTop}px; height: {SIMPLIFIED.loopHeader}px;"
		title={[header.title, header.detail].filter(Boolean).join(' · ')}
	>
		{#if header.module}
			<span class="shrink-0"><FlowModuleIcon module={header.module} size={12} /></span>
		{/if}
		<span
			class={twMerge(
				'font-medium truncate shrink-0 max-w-[75%]',
				selected ? 'text-accent' : 'text-emphasis'
			)}
		>
			{header.title}
		</span>
		<span class="font-mono text-primary shrink-0">{header.stepId}</span>
		{#if header.detail}
			<span class="font-mono text-hint truncate min-w-0">{header.detail}</span>
		{/if}
	</div>
</div>
