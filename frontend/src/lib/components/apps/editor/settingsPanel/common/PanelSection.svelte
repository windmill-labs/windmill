<script lang="ts">
	import { classNames } from '$lib/utils'
	import Tooltip from '../../../../Tooltip.svelte'

	export let title: string
	export let noPadding: boolean = false
	export let fullHeight: boolean = true
	export let titlePadding: string = ''
	export let tooltip = ''
	export let documentationLink: string | undefined = undefined
	export let id: string | undefined = undefined
</script>

<div
	class={classNames(
		$$props.class,
		'flex flex-col gap-2 items-start',
		noPadding ? '' : 'p-3',
		fullHeight ? 'h-full' : ''
	)}
	{id}
>
	<div class="flex justify-between flex-wrap items-center w-full gap-1">
		<div class="text-xs inline-flex items-center font-semibold text-primary {titlePadding} gap-1">
			<span class="truncate">
				{title}
			</span>
			{#if tooltip}
				<Tooltip {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</div>
		<slot name="action" />
	</div>
	<slot />
</div>
