<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import { Code2, LayoutDashboard, Wind } from 'lucide-svelte'
	import Icon from 'svelte-awesome'

	export let kind: 'script' | 'flow' | 'app'

	const color = {
		script: 'bg-blue-50 border-blue-200',
		flow: 'bg-[#f0fdfa] border-[#99f6e4]',
		app: 'bg-[#fff7ed] border-orange-300'
	}[kind]

	const iconColor = {
		script: '#60A5FA',
		flow: '#14b8a6',
		app: '#fb923c'
	}[kind]
</script>

<div class={classNames('rounded-md p-1 flex justify-center items-center border', color)}>
	{#if kind === 'flow'}
		<span class="ml-1 mb-0.5 -mt-0.5">
			<Icon data={faBarsStaggered} scale={1.1} class="mr-0.5 text-[#14b8a6]" />
		</span>
	{:else if kind === 'app'}
		<LayoutDashboard size={24} color={iconColor} />
	{:else if kind === 'script'}
		<Code2 size={24} color={iconColor} />
	{/if}
</div>
