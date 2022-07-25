<script lang="ts">
	import { derived, writable } from 'svelte/store'
	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'

	export let pickableProperties: Object = {}

	const EMPTY_STRING = ''
	const search = writable(EMPTY_STRING)

	const propsFiltered = derived(search, ($search: string) => {
		if ($search === EMPTY_STRING) {
			return pickableProperties
		}
		return keepByKey(pickableProperties, $search)
	})
</script>

<div
	class="relative p-3 bg-white rounded-lg border border-gray-200 shadow-md max-h-max overflow-y-scroll w-full"
>
	<div class="overflow-y-auto max-h-96">
		<input
			bind:value={$search}
			class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg block w-full p-2 mb-2"
			placeholder="Search prop..."
		/>
		<ObjectViewer json={$propsFiltered} on:select />
	</div>
</div>
