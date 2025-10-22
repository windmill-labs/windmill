<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Code2, LayoutDashboard } from 'lucide-svelte'

	export let kind: 'script' | 'flow' | 'app' | 'raw_app'
	export let href: string = '#'
</script>

<a {href} class="flex justify-center items-center">
	{#if kind === 'flow'}
		<BarsStaggered size={16} class="text-teal-500" />
	{:else if kind === 'app' || kind === 'raw_app'}
		<LayoutDashboard size={16} class="text-orange-500" />
	{:else if kind === 'script'}
		<Code2 size={16} class="text-blue-500" />
	{/if}
</a>
