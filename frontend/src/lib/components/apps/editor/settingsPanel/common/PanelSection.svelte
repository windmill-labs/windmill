<script lang="ts">
	import { classNames } from '$lib/utils'
	import Tooltip from '../../../../Tooltip.svelte'

	export let title: string
	export let noPadding: boolean = false
	export let titlePadding: string = ''
	export let tooltip = ''
	export let documentationLink: string | undefined = undefined
</script>

<div
	class={classNames(
		$$props.class,
		'flex flex-col h-full gap-2 items-start',
		noPadding ? '' : 'p-3'
	)}
>
	<div class="flex justify-between flex-wrap items-center w-full gap-1">
		<div class="text-sm inline-flex items-center font-extrabold {titlePadding} gap-1">
			<span class="truncate">
				{title}
			</span>
			{#if tooltip}
				<Tooltip light {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</div>
		<slot name="action" />
	</div>
	<slot />
</div>
