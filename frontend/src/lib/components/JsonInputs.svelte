<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let updateOnBlur = true
	export let placeholder =
		'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}'
	export let selected: boolean = false

	let pendingJson = ''
	let simpleEditor: SimpleEditor | undefined = undefined
	let focusTrap: HTMLElement | undefined

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

	export function resetSelected(dispatchEvent?: boolean) {
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			focusTrap?.focus()
			resetSelected(true)
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- Add a hidden button that can receive focus -->
<button bind:this={focusTrap} class="sr-only" tabindex="-1" aria-hidden="true">Focus trap</button>

<div class="h-full">
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
		class="h-full json-inputs-editor"
		{placeholder}
	/>
</div>

<style>
	:global(.json-inputs-editor .monaco-editor .suggest-widget) {
		z-index: 200000 !important;
	}
</style>
