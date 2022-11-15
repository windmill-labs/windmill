<script lang="ts">
	import Icon from 'svelte-awesome'
	import { faDisplay } from '@fortawesome/free-solid-svg-icons'
	import { faWpforms } from '@fortawesome/free-brands-svg-icons'
	import { createEventDispatcher } from 'svelte'

	const defaultProps = {
		horizontalAlignement: 'center',
		verticalAlignement: 'center',
		title: 'My title',
		description: 'My description',
		configSchema: undefined,
		inputs: {},
		componentInputs: {}
	}

	let items = [
		{
			...defaultProps,
			// Used by the dnd library, should be replaced by unique id
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInputs: {
				result: {
					id: undefined,
					name: undefined,
					type: 'output',
					defaultValue: undefined
				}
			}
		},
		{
			...defaultProps,
			// Used by the dnd library, should be replaced by unique id
			id: 'runformcomponent',
			type: 'runformcomponent'
		}
	]

	const displayData = {
		displaycomponent: {
			name: 'Display component',
			icon: faDisplay
		},
		runformcomponent: {
			name: 'Run form',
			icon: faWpforms
		}
	}

	const dispatch = createEventDispatcher()
</script>

<section class="grid grid-cols-3 gap-2 p-2">
	{#each items as item (item.id)}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md"
			on:click={() => dispatch('pick', item)}
		>
			<Icon data={displayData[item.type].icon} scale={1.6} />
			<div class="text-xs">{displayData[item.type].name}</div>
		</div>
	{/each}
</section>
