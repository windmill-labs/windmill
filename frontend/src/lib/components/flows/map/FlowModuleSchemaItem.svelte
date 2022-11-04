<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { faBed, faRepeat, faStop, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let color: 'blue' | 'orange' | 'indigo' = 'blue'
	export let isFirst: boolean = false
	export let isLast: boolean = false
	export let hasLine: boolean = true
	export let selected: boolean = false
	export let deletable: boolean = false
	export let retry: boolean = false
	export let earlyStop: boolean = false
	export let suspend: boolean = false
	export let id: string | undefined = undefined
	export let label: string

	const margin = isLast ? '' : isFirst ? 'mb-0.5' : 'my-0.5'
	const dispatch = createEventDispatcher<{ delete: CustomEvent<MouseEvent> }>()
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="flex relative" on:click>
	<div
		class={classNames(
			'flex mr-2 ml-0.5',
			hasLine ? 'line' : '',
			isFirst ? 'justify-center items-start' : 'justify-center items-center'
		)}
	>
		<div
			class={classNames(
				'flex justify-center items-center w-6 h-6 border rounded-full text-xs font-bold',
				color === 'blue'
					? 'bg-blue-200 text-blue-800'
					: color === 'orange'
					? 'bg-orange-200 text-orange-800'
					: 'bg-blue-100 text-blue-600',
				''
			)}
		>
			<slot name="icon" />
		</div>
	</div>
	<div
		class={classNames(
			'w-full flex overflow-hidden rounded-sm cursor-pointer',
			selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : '',
			margin
		)}
	>
		<div class="absolute text-sm right-14 -bottom-3 flex flex-row gap-1">
			{#if retry}
				<div class="bg-white rounded border text-gray-600 px-1">
					<Icon scale={0.8} data={faRepeat} />
				</div>
			{/if}
			{#if earlyStop}
				<div class="bg-white rounded border text-gray-600 px-1">
					<Icon scale={0.8} data={faStop} />
				</div>
			{/if}
			{#if suspend}
				<div class="bg-white rounded border text-gray-600 px-1">
					<Icon scale={0.8} data={faBed} />
				</div>
			{/if}
		</div>
		<div
			class="flex justify-between items-center w-full overflow-hidden border p-2 bg-white text-sm"
		>
			<div class="flex-1 truncate">{label}</div>
			<div class="flex items-center space-x-2">
				{#if id}
					<Badge color="indigo">{id}</Badge>
				{/if}
				{#if deletable}
					<Button
						on:click={(event) => dispatch('delete', event)}
						startIcon={{ icon: faTrashAlt, classes: 'text-gray-500' }}
						iconOnly={true}
						color="light"
						variant="border"
						size="xs"
					/>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
