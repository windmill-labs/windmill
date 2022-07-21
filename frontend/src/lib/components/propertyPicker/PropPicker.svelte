<script lang="ts">
	import { derived, writable } from 'svelte/store'
	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'

	export let props: Object = {}

	const EMPTY_STRING = ''
	const search = writable(EMPTY_STRING)

	$: propsFiltered = derived(search, ($search: string) => {
		if ($search === EMPTY_STRING) {
			return props
		}
		return keepByKey(props, $search)
	})
</script>

<div class="relative">
	<div class="p-3 bg-white rounded-lg border border-gray-200 shadow-md">
		<input
			bind:value={$search}
			class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg block w-full p-2 mb-2"
			placeholder="Search prop..."
		/>
		<ObjectViewer json={$propsFiltered} on:select />
	</div>
</div>
