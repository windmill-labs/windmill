<script lang="ts">
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { ExternalLink } from 'lucide-svelte'

	export let light = false
	export let scale = 0.8
	export let wrapperClass = ''
	export let placement: PopoverPlacement | undefined = undefined
	export let documentationLink: string | undefined = undefined
</script>

<Popover notClickable {placement} class={wrapperClass}>
	<Icon
		class="{light
			? 'text-gray-400'
			: ' text-gray-500'} font-thin inline-block align-middle w-4 {$$props.class}"
		data={faInfoCircle}
		{scale}
	/>
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
