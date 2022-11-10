<script lang="ts">
	import { flip } from 'svelte/animate'
	import { dndzone, TRIGGERS, SHADOW_ITEM_MARKER_PROPERTY_NAME } from 'svelte-dnd-action'
	import Icon from 'svelte-awesome'
	import { faDisplay } from '@fortawesome/free-solid-svg-icons'
	import { faWpforms } from '@fortawesome/free-brands-svg-icons'
	import { getNextId } from '$lib/components/flows/flowStateUtils'

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

	const flipDurationMs = 300
	let shouldIgnoreDndEvents = false

	function handleDndConsider(e) {
		const { trigger, id } = e.detail.info
		if (trigger === TRIGGERS.DRAG_STARTED) {
			const idx = items.findIndex((item) => item.id === id)

			e.detail.items = e.detail.items.filter((item) => !item[SHADOW_ITEM_MARKER_PROPERTY_NAME])

			e.detail.items.splice(idx, 0, {
				...items[idx],
				id: getNextId(e.detail.items.map((item) => item.id)),
				// @ts-ignore
				type: items[idx].type
			})
			items = e.detail.items
			shouldIgnoreDndEvents = true
		} else if (!shouldIgnoreDndEvents) {
			items = e.detail.items
		} else {
			items = [...items]
		}
	}

	function handleDndFinalize(e) {
		if (!shouldIgnoreDndEvents) {
			items = e.detail.items
		} else {
			items = [...items]
			shouldIgnoreDndEvents = false
		}
	}
</script>

<section
	use:dndzone={{ items, flipDurationMs, type: 'component' }}
	on:consider={handleDndConsider}
	on:finalize={handleDndFinalize}
	class="grid grid-cols-2 gap-2 p-2"
>
	{#each items as item (item.id)}
		<div
			class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md"
			animate:flip={{ duration: flipDurationMs }}
		>
			<Icon data={displayData[item.type].icon} scale={1.6} />
			<div class="text-xs">{displayData[item.type].name}</div>
		</div>
	{/each}
</section>
