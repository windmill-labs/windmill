<script lang="ts">
	import { classNames } from '$lib/utils'

	export let title: string
</script>

<div class={classNames('flex flex-col gap-2 items-start p-4')}>
	<div class="flex justify-between items-center w-full">
		<div class="text-sm font-bold">{title}</div>
		<slot name="action" />
	</div>
	<slot />
</div>
