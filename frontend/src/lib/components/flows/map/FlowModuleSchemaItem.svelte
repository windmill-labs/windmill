<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faClose, faCross, faTrash, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let color: 'blue' | 'orange' = 'blue'
	export let isFirst: boolean = false
	export let isLast: boolean = false
	export let hasLine: boolean = true
	export let selected: boolean = false
	export let deletable: boolean = false

	const margin = isLast ? '' : isFirst ? 'mb-2' : 'my-2'
	const dispatch = createEventDispatcher()
</script>

<div class="flex" on:click>
	<div class={classNames('flex  items-center mr-2 w-8 justify-center', hasLine ? 'line' : '')}>
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
	<div
		class={classNames(
			'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex justify-between items-center',
			margin,
			selected ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
		)}
	>
		<slot name="content" />
		{#if deletable}
			<button
				type="button"
				on:click={() => dispatch('delete')}
				class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-md text-xs px-2 py-1"
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
