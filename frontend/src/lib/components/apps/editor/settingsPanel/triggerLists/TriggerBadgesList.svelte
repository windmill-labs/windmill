<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { DependencyBadge } from './triggerListUtils'

	export let badges: DependencyBadge
	export let onClick: boolean = false
	export let onLoad: boolean = false

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		yellow: 'text-yellow-800 border-yellow-600 bg-yellow-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		orange: 'text-orange-800 border-orange-600 bg-orange-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	{#if badges.length === 0 && !onLoad && !onClick}
		<p class="text-xs font-semibold text-slate-800 ">
			This script has no triggers. It will never run.
		</p>
	{:else}
		<p class="text-xs font-semibold text-slate-800 ">Triggers</p>
		<div class="flex flex-row gap-2 flex-wrap">
			{#each badges as badge}
				<span class={classNames(badgeClass, colors[badge.color])}>{badge.label}</span>
			{/each}
			{#if onLoad}
				<span class={classNames(badgeClass, colors['green'])}>On load</span>
			{/if}
			{#if onClick}
				<span class={classNames(badgeClass, colors['blue'])}>On click</span>
			{/if}
		</div>
	{/if}
</div>
