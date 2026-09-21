<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher, untrack } from 'svelte'

	const dispatch = createEventDispatcher()

	interface Props {
		updateOnBlur?: boolean
		placeholder?: string
		selected?: boolean
		/** Content the editor opens with, and keeps following while the buffer is untouched — so a
		 * payload nobody has typed into tracks the schema instead of going stale. The first edit
		 * hands the buffer to the user and later changes stop overwriting it. */
		initialCode?: string
	}

	let {
		updateOnBlur = true,
		placeholder = 'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}',
		selected = false,
		initialCode = ''
	}: Props = $props()

	let pendingJson = $state(untrack(() => initialCode))
	// The last content this component wrote, kept only to skip a reseed that would replace the
	// buffer with what it already holds — `setValue` resets the cursor and the undo stack.
	let seededCode = untrack(() => initialCode)
	// Latched from Monaco's own change event, never from `pendingJson`: that trails the buffer by
	// SimpleEditor's debounce, a window in which typed text still looks like the seeded payload
	// and a reseed lands on top of it.
	let userEdited = false
	let simpleEditor: SimpleEditor | undefined = $state(undefined)
	let focusTrap: HTMLElement | undefined = $state()

	$effect(() => {
		const next = initialCode
		untrack(() => {
			if (next !== seededCode && !userEdited) {
				seed(next)
			}
		})
	})

	// `SimpleEditor.setCode` cancels the change burst its own `setValue` opens, so reseeding
	// never dispatches `select` — the payload reaches `args` only when the user edits it.
	function seed(code: string) {
		seededCode = code
		userEdited = false
		pendingJson = code
		simpleEditor?.setCode(code)
	}

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

	/** Authoritative overwrite: replaces the buffer whether or not it has been typed into, and
	 * re-establishes it as the content to keep following. */
	export function setCode(code: string) {
		seed(code)
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

<svelte:window onkeydown={handleKeydown} />

<!-- Add a hidden button that can receive focus -->
<button bind:this={focusTrap} class="sr-only" tabindex="-1" aria-hidden="true">Focus trap</button>

<div class="h-full rounded-md border">
	<SimpleEditor
		bind:this={simpleEditor}
		on:input={() => (userEdited = true)}
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
