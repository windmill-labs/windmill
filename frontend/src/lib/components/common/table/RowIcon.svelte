<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Code2, LayoutDashboard, Wind } from 'lucide-svelte'

	export let kind: 'script' | 'flow' | 'app'

	const icon = {
		script: Code2,
		flow: Wind,
		app: LayoutDashboard
	}[kind]

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
	<svelte:component this={icon} size={18} color={iconColor} />
</div>
