<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let color: 'blue' | 'orange' = 'blue'
	export let isFirst: boolean = false
	export let isLast: boolean = false
	export let hasLine: boolean = true
	export let selected: boolean = false
	export let deletable: boolean = false

	const margin = isLast ? '' : isFirst ? 'mb-0.5' : 'my-0.5'
	const dispatch = createEventDispatcher()
</script>

<div class="flex" on:click>
	<div
		class={classNames(
			'flex  mr-2 w-8 ',
			hasLine ? 'line' : '',
			isFirst ? 'justify-center items-start' : 'justify-center items-center'
		)}
	>
		<div
			class={classNames(
				'flex justify-center items-center w-6 h-6 border rounded-full text-xs font-bold',
				color === 'blue' ? 'bg-blue-100 text-blue-400' : 'bg-orange-100 text-orange-400',
				margin
			)}
		>
			<slot name="icon" />
		</div>
	</div>
	<div
		class={classNames(
			'border w-full rounded-sm p-2 bg-white text-sm cursor-pointer flex justify-between items-center space-x-2',
			margin,
			selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : ''
		)}
	>
		<slot name="content" />
		{#if deletable}
			<button
				type="button"
				on:click={() => dispatch('delete')}
				class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-sm text-xs px-2 py-1"
			>
				<Icon data={faTrashAlt} scale={0.8} />
			</button>
		{/if}
	</div>
</div>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
