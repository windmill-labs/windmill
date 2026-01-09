<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Boxes, Code2, DollarSign, LayoutDashboard } from 'lucide-svelte'

	export let kind: 'script' | 'flow' | 'app' | 'raw_app' | 'resource' | 'variable'
</script>

<div class="flex justify-center items-center">
	{#if kind === 'flow'}
		<BarsStaggered size={16} class="text-teal-500" />
	{:else if kind === 'app' || kind === 'raw_app'}
		<LayoutDashboard size={16} class="text-orange-500" />
	{:else if kind === 'script'}
		<Code2 size={16} class="text-blue-500" />
	{:else if kind === 'variable'}
		<DollarSign size={16} class="text-gray-400" />
	{:else if kind === 'resource'}
		<Boxes size={16} class="text-gray-400" />
	{:else}
		<div class="w-[16px]"></div>
	{/if}
</div>
