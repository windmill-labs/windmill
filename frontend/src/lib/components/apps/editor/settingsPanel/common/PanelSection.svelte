<script lang="ts">
	import { classNames } from '$lib/utils'

	export let title: string
	export let smallPadding: boolean = false
</script>

<div class={classNames('flex flex-col gap-2 items-start', smallPadding ? 'p-2' : 'p-4')}>
	<div class="flex justify-between items-center w-full">
		<div class="text-sm font-extrabold">{title}</div>
		<slot name="action" />
	</div>
	<slot />
</div>
