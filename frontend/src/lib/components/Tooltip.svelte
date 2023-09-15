<script lang="ts">
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { ExternalLink, InfoIcon } from 'lucide-svelte'

	export let light = false
	export let wrapperClass = ''
	export let placement: PopoverPlacement | undefined = undefined
	export let documentationLink: string | undefined = undefined
	export let small = false
</script>

<Popover notClickable {placement} class={wrapperClass}>
	<div
		class="inline-flex w-3 mx-0.5 {light
			? 'text-tertiary-inverse'
			: 'text-tertiary'} {$$props.class} relative"
	>
		<InfoIcon class="{small ? 'bottom-0' : '-bottom-0.5'} absolute" size={small ? 12 : 16} />
	</div>
	<svelte:fragment slot="text">
		<slot />
		{#if documentationLink}
			<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
				<div class="flex flex-row gap-2 mt-4">
					See documentation
					<ExternalLink size="16" />
				</div>
			</a>
		{/if}
	</svelte:fragment>
</Popover>
