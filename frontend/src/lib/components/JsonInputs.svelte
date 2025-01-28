<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let updateOnBlur = true

	let pendingJson = ''
	let simpleEditor: SimpleEditor | undefined = undefined

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

	export function setCode(code: string) {
		simpleEditor?.setCode(code)
	}
</script>

<SimpleEditor
	bind:this={simpleEditor}
	on:focus={() => {
		if (updateOnBlur) {
			dispatch('focus')
			updatePayloadFromJson(pendingJson)
		}
	}}
	on:blur={async () => {
		if (updateOnBlur) {
			dispatch('blur')
			setTimeout(() => {
				updatePayloadFromJson('')
			}, 100)
		}
	}}
	on:change={(e) => {
		if (e.detail?.code !== undefined) {
			updatePayloadFromJson(e.detail.code)
		}
	}}
	bind:code={pendingJson}
	lang="json"
	class="h-full"
	placeholder={'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}'}
/>
