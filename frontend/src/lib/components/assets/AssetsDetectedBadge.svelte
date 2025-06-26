<script lang="ts">
	import { clone, pluralize } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { Pyramid } from 'lucide-svelte'

	const { assets }: { assets: string[] } = $props()

	let prevAssets = $state<string[]>(assets ? clone(assets) : [])
	let blueBgDiv: HTMLDivElement | undefined = $state()

	$effect(() => {
		if (!assets) return

		if (!deepEqual(assets, prevAssets)) {
			prevAssets = clone(assets)
			// Replay animation
			if (blueBgDiv) {
				blueBgDiv.style.animation = 'none'
				blueBgDiv.offsetHeight /* trigger reflow */
				blueBgDiv.style.animation = ''
			}
		}
	})
</script>

{#if assets.length > 0}
	<div
		class="text-xs z-50 flex text-tertiary items-center gap-2 px-2 py-1 bg-surface rounded-md border relative"
	>
		<div
			bind:this={blueBgDiv}
			class="absolute bg-blue-400/30 inset-0 rounded-md -z-50 opacity-0 animate-fade-out"
		></div>
		<Pyramid size={16} />
		{pluralize(assets.length, 'asset')}
	</div>
{/if}
