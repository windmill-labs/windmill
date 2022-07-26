<script lang="ts">
	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'

	export let pickableProperties: Object = {}

	const EMPTY_STRING = ''
	let search = ''

	$: propsFiltered =
		search === EMPTY_STRING ? pickableProperties : keepByKey(pickableProperties, search)
</script>

<div
	class="relative p-3 bg-white rounded-lg border border-gray-200 shadow-md max-h-max overflow-y-scroll"
>
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
