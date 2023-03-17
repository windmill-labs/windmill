<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { DependencyBadge } from './triggerListUtils'

	export let badges: DependencyBadge

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		yellow: 'text-yellow-800 border-yellow-600 bg-yellow-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	{#if badges.length === 0}
		<p class="text-xs font-semibold text-slate-800 ">No executions</p>
	{:else}
		<p class="text-xs font-semibold text-slate-800 ">Executions ({badges.length})</p>
		<div class="flex flex-row gap-1">
			{#each badges as badge}
				<span class={classNames(badgeClass, colors[badge.color])}>{badge.label}</span>
			{/each}
		</div>
	{/if}
</div>
