<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, RichConfigurations } from '../../types'
	import 'quill/dist/quill.snow.css'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let render: boolean

	let editor
	let quill: any
	export let toolbarOptions = [
		[{ header: 1 }, { header: 2 }, 'blockquote', 'link', 'image', 'video'],
		['bold', 'italic', 'underline', 'strike'],
		[{ list: 'ordered' }, { list: 'ordered' }],
		[{ align: [] }],
		['clean']
	]

	const { worldStore, componentControl, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	let placeholder: string | undefined = undefined
	let defaultValue: string | undefined = undefined

	let outputs = initOutput($worldStore, id, {
		result: ''
	})

	$: if (!render) {
		quill = undefined
	}

	$: if (!quill && render) {
		loadQuill()
	}

	async function loadQuill() {
		const { default: Quill } = await import('quill')

		quill = new Quill(editor, {
			modules: {
				toolbar: toolbarOptions
			},
			theme: 'snow',
			placeholder: placeholder
		})

		if (defaultValue) {
			quill.root.innerHTML = defaultValue
		}

		quill.on('text-change', function (delta, oldDelta, source) {
			setOutput()
		})
	}

	$componentControl[id] = {
		setValue(nvalue: string) {
			if (quill) {
				quill.root.innerHTML = nvalue
				setOutput()
			}
		}
	}

	function setOutput() {
		if (quill) {
			outputs?.result.set(quill.root.innerHTML)
		}
	}

	$: handleDefault(defaultValue)

	function handleDefault(defaultValue: string | undefined) {
		if (quill) {
			quill.root.innerHTML = defaultValue
			setOutput()
		}
	}
</script>

<InputValue key="placeholder" {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue key="value" {id} input={configuration.defaultValue} bind:value={defaultValue} />

<InitializeComponent {id} />
{#if render}
	<div
		class="editor-wrapper h-full flex-col flex max-h-full overflow-hidden"
		on:pointerdown|stopPropagation={() => {
			if (!$connectingInput.opened) {
				$selectedComponent = [id]
			}
		}}
	>
		<div bind:this={editor}></div>
	</div>
{/if}

<style lang="postcss">
	:global(.ql-toolbar) {
		@apply rounded-t-md;
	}

	:global(.dark .ql-toolbar) {
		@apply border-gray-500;
	}

	:global(.ql-toolbar .ql-stroke) {
		fill: none;
		@apply stroke-primary rounded-t-md;
	}

	:global(.ql-toolbar .ql-fill) {
		@apply fill-primary bg-red-500;
		stroke: none;
	}

	:global(.ql-toolbar .ql-picker) {
		@apply text-primary;
	}

	:global(.ql-container) {
		@apply text-primary rounded-b-md;
	}

	:global(.dark .ql-container) {
		@apply border-gray-500;
	}

	:global(.dark .ql-container > .ql-editor.ql-blank::before) {
		@apply text-gray-500;
	}
</style>
