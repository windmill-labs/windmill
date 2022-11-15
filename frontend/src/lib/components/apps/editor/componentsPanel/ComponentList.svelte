<script lang="ts">
	import { flip } from 'svelte/animate'
	import { dndzone } from 'svelte-dnd-action'
	import Icon from 'svelte-awesome'
	import { displayData } from '../../utils'

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

	let x = [
		{
			...defaultProps,
			// Used by the dnd library, should be replaced by unique id
			id: 'textcomponent',
			type: 'textcomponent'
		},
		{
			...defaultProps,
			id: 'buttoncomponent',
			type: 'buttoncomponent'
		},
		{
			...defaultProps,
			id: 'imagecomponent',
			type: 'imagecomponent'
		},
		{
			...defaultProps,
			id: 'inputcomponent',
			type: 'inputcomponent'
		},
		{
			...defaultProps,
			id: 'selectcomponent',
			type: 'selectcomponent'
		},
		{
			...defaultProps,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent'
		},
		{
			...defaultProps,
			id: 'radiocomponent',
			type: 'radiocomponent'
		}
	]

	let charts = [
		{
			...defaultProps,
			// Used by the dnd library, should be replaced by unique id
			id: 'piechartcomponent',
			type: 'piechartcomponent'
		},
		{
			...defaultProps,
			id: 'barchartcomponent',
			type: 'barchartcomponent'
		}
	]

	const flipDurationMs = 300
</script>

<div class="px-4 pt-4 text-sm font-semibold">Component</div>

<section
	use:dndzone={{ items, flipDurationMs, type: 'component' }}
	class="grid grid-cols-3 gap-4 p-4"
>
	{#each items as item (item.id)}
		<div
			class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-100"
			animate:flip={{ duration: flipDurationMs }}
		>
			<Icon data={displayData[item.type].icon} scale={1.6} class="text-blue-800" />
			<div class="text-xs">{displayData[item.type].name}</div>
		</div>
	{/each}
</section>

<div class="px-4 pt-4 text-sm font-semibold">Components</div>
<section
	use:dndzone={{ items: x, flipDurationMs, type: 'component' }}
	class="grid grid-cols-3 gap-4 p-4"
>
	{#each x as item (item.id)}
		<div
			class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-100"
			animate:flip={{ duration: flipDurationMs }}
		>
			<Icon data={displayData[item.type].icon} scale={1.6} class="text-blue-800" />
			<div class="text-xs">{displayData[item.type].name}</div>
		</div>
	{/each}
</section>

<div class="px-4 pt-4 text-sm font-semibold">Charts</div>
<section
	use:dndzone={{ items: charts, flipDurationMs, type: 'component' }}
	class="grid grid-cols-3 gap-4 p-4"
>
	{#each charts as item (item.id)}
		<div
			class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md scale-100 hover:scale-105 ease-in duration-100"
			animate:flip={{ duration: flipDurationMs }}
		>
			<Icon data={displayData[item.type].icon} scale={1.6} class="text-blue-800" />
			<div class="text-xs">{displayData[item.type].name}</div>
		</div>
	{/each}
</section>
