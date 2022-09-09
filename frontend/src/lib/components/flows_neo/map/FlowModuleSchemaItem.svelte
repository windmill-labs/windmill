<script lang="ts">
	import { classNames } from '$lib/utils'

	export let color: 'blue' | 'orange' = 'blue'
	export let isFirst: boolean = false
	export let isLast: boolean = false

	const margin = isLast ? '' : isFirst ? 'mb-2' : 'my-2'
</script>

<div class="flex" on:click>
	<div
		class={classNames('flex  items-center line mr-2 w-8', isLast ? 'flex-col-reverse' : 'flex-col')}
	>
		<div
			class={classNames(
				'flex items-center justify-center w-6 h-6 border rounded-full text-xs font-bold',
				color === 'blue' ? 'bg-blue-300 text-blue-600' : 'bg-orange-300 text-orange-600',
				margin
			)}
		>
			<slot name="icon" />
		</div>
	</div>
	<div class={classNames('border w-full rounded-md p-2 bg-white text-sm', margin)}>
		<slot name="content" />
	</div>
</div>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
