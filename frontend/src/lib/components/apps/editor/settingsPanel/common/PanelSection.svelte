<script lang="ts">
	import { classNames } from '$lib/utils'

	export let title: string
	export let root: boolean = true
</script>

<div class={classNames('flex flex-col gap-2 items-start', root ? 'p-4' : 'py-4')}>
	<div class="flex justify-between items-center w-full">
		<div class="text-xs font-bold">{title}</div>
		<slot name="action" />
	</div>
	<slot />
</div>
