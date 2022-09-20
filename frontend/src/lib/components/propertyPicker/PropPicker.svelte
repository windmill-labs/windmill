<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'

	export let pickableProperties: Object = {}

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig, clearFocus } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: propsFiltered =
		search === EMPTY_STRING ? pickableProperties : keepByKey(pickableProperties, search)
</script>

<div class="h-full space-y-2 flex flex-col">
	<div class="flex justify-between items-center h-8">
		<span class="font-bold text-sm">Context</span>
		<div class="flex space-x-2 items-center">
			{#if $propPickerConfig}
				<span
					class="flex items-center bg-yellow-100 text-yellow-800 text-xs font-semibold px-2 py-1 rounded dark:bg-green-200 dark:text-green-900"
				>
					{`Selected input: ${$propPickerConfig?.propName}`}
				</span>
				<span
					class="flex items-center bg-green-100 text-green-800 text-xs font-semibold px-2 py-1 rounded dark:bg-green-200 dark:text-green-900"
				>
					{`Mode: ${$propPickerConfig?.insertionMode}`}
				</span>
				<button
					class="border px-2 py-1 text-xs rounded-md flex items-center hover:bg-gray-50 hover:text-gray-900"
					on:click={() => clearFocus()}
				>
					<Icon data={faClose} class="mr-2" scale={0.8} />
					Deselect
				</button>
			{/if}
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
