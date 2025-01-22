<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	let pendingJson: string

	function updatePayloadFromJson(jsonInput: string) {
		if (jsonInput === undefined || jsonInput === null || jsonInput.trim() === '') {
			dispatch('select', undefined)
			return
		}
		try {
			const parsed = JSON.parse(jsonInput)
			dispatch('select', parsed)
		} catch (error) {
			dispatch('select', undefined)
		}
	}
</script>

<SimpleEditor
	on:focus={() => {
		dispatch('focus')
		updatePayloadFromJson(pendingJson)
	}}
	on:blur={async () => {
		dispatch('blur')
		setTimeout(() => {
			updatePayloadFromJson('')
		}, 100)
	}}
	on:change={(e) => {
		updatePayloadFromJson(e.detail.code)
	}}
	bind:code={pendingJson}
	lang="json"
	class="h-full"
	placeholder={'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}'}
/>
