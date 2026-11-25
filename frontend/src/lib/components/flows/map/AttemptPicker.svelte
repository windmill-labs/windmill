<!--
@component
A retried step's attempts: an optional run count, the retry policy, a search past
`SEARCH_FROM` attempts, then the attempts. The Retries submenu, or the whole settings menu
of a step whose only setting is retries.
-->
<script lang="ts" module>
	export const SEARCH_FROM = 10
</script>

<script lang="ts">
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { Check } from 'lucide-svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import type { Job } from '$lib/gen'

	type Attempt = { index: number; success: boolean | undefined; selected: boolean }

	interface Props {
		policy: string
		attempts: Attempt[]
		onPick: (index: number) => void
		meltItem: MenubarMenuElements['item']
		close: (afterClose?: () => void) => void
		/** Shows the run count above the policy, for a menu with no Retries row to carry it. */
		showCount?: boolean
		class?: string
	}

	let {
		policy,
		attempts,
		onPick,
		meltItem,
		showCount = false,
		class: className = 'w-56'
	}: Props = $props()

	let query = $state('')

	function status(success: boolean | undefined): string {
		return success === true ? 'Succeeded' : success === false ? 'Failed' : 'Running'
	}

	// A number matches the attempt numbers containing it; words match the status.
	let shown = $derived.by(() => {
		const q = query.trim().toLowerCase().replace(/^#/, '')
		if (!q) return attempts
		return attempts.filter(
			(a) => String(a.index + 1).includes(q) || status(a.success).toLowerCase().includes(q)
		)
	})

	// JobStatusIcon reads only `success` (finished) or `running`, which is all an attempt list has.
	function attemptJob(success: boolean | undefined): Job {
		return (success === undefined ? { running: true } : { success }) as unknown as Job
	}
</script>

<div class="{className} py-1">
	{#if showCount}
		<p class="px-4 pt-1.5 text-xs font-medium text-orange-500">{attempts.length} attempts</p>
	{/if}
	{#if policy}
		<p class="px-4 pt-1.5 pb-1 text-2xs text-secondary">{policy}</p>
	{/if}
	{#if attempts.length > SEARCH_FROM}
		<!-- Keys typed here must not reach the menu, whose typeahead would move the highlight. -->
		<div class="px-2 pb-1" onkeydown={(e) => e.key !== 'Escape' && e.stopPropagation()} role="none">
			<TextInput bind:value={query} size="sm" inputProps={{ placeholder: 'Search attempts' }} />
		</div>
	{/if}
	<div class="max-h-64 overflow-y-auto">
		{#each shown as a (a.index)}
			<MenuItem
				item={meltItem}
				onClick={() => onPick(a.index)}
				class="px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full data-[highlighted]:bg-surface-hover flex flex-row gap-2 items-center rounded-sm"
			>
				<JobStatusIcon job={attemptJob(a.success)} />
				<p class="grow text-left">#{a.index + 1}</p>
				{#if a.selected}
					<Check size={14} class="text-primary" />
				{/if}
			</MenuItem>
		{:else}
			<p class="px-4 py-2 text-xs text-secondary">No attempt matches</p>
		{/each}
	</div>
</div>
