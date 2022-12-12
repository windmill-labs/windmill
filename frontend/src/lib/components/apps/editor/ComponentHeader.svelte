<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppComponent } from '../types'
	import { displayData } from '../utils'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let component: AppComponent
	export let selected: boolean

	const dispatch = createEventDispatcher()
</script>

<span
	class={classNames(
		'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5',
		selected ? 'bg-indigo-500' : 'bg-gray-500'
	)}
>
	{displayData[component.type].name} - {component.id}
</span>
<button
	class={classNames(
		'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5 right-0  cursor-pointer',
		'bg-gray-600 hover:bg-gray-800'
	)}
	on:click={() => {
		dispatch('delete')
	}}
>
	<X size={16} />
</button>
