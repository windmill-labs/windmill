<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import { Code2, LayoutDashboard } from 'lucide-svelte'
	import Icon from 'svelte-awesome'

	export let kind: 'script' | 'flow' | 'app' | 'raw_app'
	export let href: string = '#'

	const color = {
		script: 'bg-blue-50 border-blue-200 dark:bg-transparent dark:border-blue-900',
		flow: 'bg-[#f0fdfa] border-[#99f6e4] dark:bg-transparent dark:border-teal-900',
		app: 'bg-[#fff7ed] border-orange-300 dark:bg-transparent dark:border-orange-900',
		raw_app: 'bg-[#fff7ed] border-orange-300 dark:bg-transparent dark:border-orange-900'
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
		<span class="ml-1 mb-0.5 -mt-0.5">
			<Icon data={faBarsStaggered} scale={1} class="mr-0.5 text-[#14b8a6]" />
		</span>
	{:else if kind === 'app' || kind === 'raw_app'}
		<LayoutDashboard size={20} color={iconColor} />
	{:else if kind === 'script'}
		<Code2 size={20} color={iconColor} />
	{/if}
</a>
