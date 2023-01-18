<script lang="ts">
	import { classNames } from '$lib/utils'
	import Tooltip from '../../../../Tooltip.svelte'

	export let title: string
	export let smallPadding: boolean = false
	export let noPadding: boolean = false
	export let titlePadding: string = ''
	export let tooltip = ''
</script>

<div
	class={classNames(
		$$props.class,
		'flex flex-col h-full gap-2 items-start',
		noPadding ? '' : smallPadding ? 'p-2' : 'p-4'
	)}
>
	<div class="flex justify-between items-center w-full gap-1">
		<div class="text-sm font-extrabold {titlePadding}">
			{title}
			{#if tooltip}
				<Tooltip>
					{tooltip}
				</Tooltip>
			{/if}
		</div>
		<slot name="action" />
	</div>
	<slot />
</div>
