<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import { clickOutside } from '$lib/utils'

	const dispatch = createEventDispatcher()

	export let updateOnBlur = true
	export let placeholder =
		'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}'

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

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		const elements = document.querySelectorAll('[data-schema-picker], [data-schema-picker] *')
		return elements ? (Array.from(elements) as HTMLElement[]) : []
	}
</script>

<div
	class="h-full"
	use:clickOutside={{ capture: false, exclude: getPropPickerElements }}
	on:click_outside={() => {
		if (updateOnBlur) {
			updatePayloadFromJson('')
		}
	}}
>
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
		{placeholder}
	/>
</div>
