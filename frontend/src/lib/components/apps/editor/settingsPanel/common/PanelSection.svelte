<script lang="ts">
	import { classNames } from '$lib/utils'

	export let title: string
	export let smallPadding: boolean = false
	export let noPadding: boolean = false
	export let titlePadding: string = ''
</script>

<div
	class={classNames(
		'flex flex-col h-full gap-2 items-start',
		noPadding ? '' : smallPadding ? 'p-2' : 'p-4'
	)}
>
	<div class="flex justify-between items-center w-full gap-1">
		<div class="text-sm font-extrabold {titlePadding}">{title}</div>
		<slot name="action" />
	</div>
	<slot />
</div>
