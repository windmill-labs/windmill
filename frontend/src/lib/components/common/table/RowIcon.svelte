<script lang="ts">
	import { classNames } from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Code2, LayoutDashboard } from 'lucide-svelte'

	export let kind: 'script' | 'flow' | 'app' | 'raw_app'
	export let href: string = '#'

	const color = {
		script: 'bg-blue-50 border-blue-200 dark:bg-blue-800/20 dark:border-blue-500/70',
		flow: 'bg-[#f0fdfa] border-[#99f6e4] dark:bg-teal-800/20 dark:border-teal-600/70',
		app: 'bg-[#fff7ed] border-orange-300 dark:bg-orange-800/20 dark:border-orange-400/70',
		raw_app: 'bg-[#fff7ed] border-orange-300 dark:bg-orange-800/20 dark:border-orange-400/70'
	}[kind]

	const iconColor = {
		script: '#60A5FA',
		flow: '#14b8a6',
		app: '#fb923c',
		raw_app: '#fb923c'
	}[kind]
</script>

<a {href} class={classNames('rounded-md p-1 flex justify-center items-center border', color)}>
	{#if kind === 'flow'}
		<BarsStaggered size={20} class=" text-[#14b8a6] p-0.5" />
	{:else if kind === 'app' || kind === 'raw_app'}
		<LayoutDashboard size={20} color={iconColor} />
	{:else if kind === 'script'}
		<Code2 size={20} color={iconColor} />
	{/if}
</a>
