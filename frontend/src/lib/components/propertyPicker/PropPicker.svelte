<script lang="ts">
	import { getContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'

	export let pickableProperties: Object = {}

	const EMPTY_STRING = ''
	let search = ''

	const { focus, focused } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: propsFiltered =
		search === EMPTY_STRING ? pickableProperties : keepByKey(pickableProperties, search)
</script>

<div class="h-full space-y-2 flex flex-col">
	<div class="flex justify-between items-center">
		<span class="font-bold text-sm">Context</span>
		<div>
			{$focused ?? 'No prop focused'}
			<button on:click={() => focus(undefined)}>Exit</button>
		</div>
	</div>
	<div class="overflow-y-auto max-h-96">
		<input
			type="text"
			bind:value={search}
			class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg block p-2 mb-2"
			placeholder="Search prop..."
		/>
		<ObjectViewer json={propsFiltered} on:select />
	</div>
</div>
