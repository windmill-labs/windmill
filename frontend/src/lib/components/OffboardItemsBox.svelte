<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import type { OffboardAffectedPaths } from '$lib/gen'
	import type { Snippet } from 'svelte'
	import { flattenPaths, itemHref, kindLabel } from './offboarding-utils'

	type Props = {
		title: string
		paths: OffboardAffectedPaths
		variant?: 'default' | 'warning'
		description?: string
		headerExtra?: Snippet
	}

	let { title, paths, variant = 'default', description, headerExtra }: Props = $props()

	let expanded = $state(false)

	let isWarning = $derived(variant === 'warning')
</script>

<div
	class={isWarning
		? 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700/40 rounded-md p-3'
		: 'bg-surface-secondary rounded-md p-3'}
>
	<div class="flex items-center justify-between gap-2">
		<button class="flex items-center gap-1 text-left" onclick={() => (expanded = !expanded)}>
			{#if expanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
			<span
				class={isWarning
					? 'text-xs font-medium text-yellow-800 dark:text-yellow-100/90'
					: 'text-xs font-medium text-primary'}>{title}</span
			>
		</button>
		{#if headerExtra}
			{@render headerExtra()}
		{/if}
	</div>
	{#if description}
		<p
			class="{isWarning
				? 'text-yellow-700 dark:text-yellow-100/70'
				: 'text-tertiary'} text-xs mt-1 ml-5"
		>
			{description}
		</p>
	{/if}
	{#if expanded}
		<ul
			class="mt-1.5 ml-5 text-xs {isWarning
				? 'text-yellow-800 dark:text-yellow-100/90'
				: 'text-secondary'} max-h-40 overflow-y-auto flex flex-col gap-0.5"
		>
			{#each flattenPaths(paths) as { kind, path }}
				<li class="flex gap-1.5"
					><span
						class="{isWarning
							? 'text-yellow-600 dark:text-yellow-300'
							: 'text-tertiary'} w-24 shrink-0">{kindLabel(kind)}</span
					>{#if itemHref(kind, path)}<a
							href={itemHref(kind, path)}
							target="_blank"
							class="truncate hover:underline text-blue-500 dark:text-blue-400">{path}</a
						>{:else}<span class="truncate">{path}</span>{/if}</li
				>
			{/each}
		</ul>
	{/if}
</div>
