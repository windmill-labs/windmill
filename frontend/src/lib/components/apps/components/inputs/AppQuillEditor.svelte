<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, untrack } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, RichConfigurations } from '../../types'
	import 'quill/dist/quill.snow.css'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	let editor: HTMLElement | undefined = $state()
	let quill: any = $state()
	interface Props {
		id: string
		configuration: RichConfigurations
		render: boolean
		toolbarOptions?: any
	}

	let {
		id,
		configuration,
		render,
		toolbarOptions = [
			[{ header: 1 }, { header: 2 }, 'blockquote', 'link', 'image', 'video'],
			['bold', 'italic', 'underline', 'strike'],
			[{ list: 'ordered' }, { list: 'ordered' }],
			[{ align: [] }],
			['clean']
		]
	}: Props = $props()

	const { worldStore, componentControl, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	let placeholder: string | undefined = $state(undefined)
	let defaultValue: string | undefined = $state(undefined)

	let outputs = initOutput($worldStore, untrack(() => id), {
		result: ''
	})

	async function loadQuill() {
		const { default: Quill } = await import('quill')
		if (!editor) return

		quill = new Quill(editor, {
			modules: {
				toolbar: toolbarOptions
			},
			theme: 'snow',
			placeholder: placeholder
		})

		quill.on('text-change', function (delta, oldDelta, source) {
			setOutput()
		})

		if (defaultValue) {
			setHtml(defaultValue)
		}
	}

	// Load HTML through Quill's clipboard converter rather than assigning `root.innerHTML`:
	// Quill only keeps markup it has a blot for, and writes to the DOM bypass that mapping.
	// Quill 2 renders every list as `<ol>`, so a `<ul>` from stored content would be dropped
	// outright instead of becoming a bullet list.
	function setHtml(html: string | undefined) {
		quill.setContents(quill.clipboard.convert({ html: html ?? '', text: '' }))
	}

	$componentControl[untrack(() => id)] = {
		setValue(nvalue: string) {
			if (quill) {
				setHtml(nvalue)
				setOutput()
			}
		}
	}

	function setOutput() {
		if (quill) {
			outputs?.result.set(quill.root.innerHTML)
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		if (quill) {
			setHtml(defaultValue)
			setOutput()
		}
	}
	$effect.pre(() => {
		if (!render) {
			quill = undefined
		}
	})
	$effect.pre(() => {
		if (!quill && render) {
			untrack(() => loadQuill())
		}
	})
	$effect.pre(() => {
		;[defaultValue]
		untrack(() => handleDefault(defaultValue))
	})
</script>

<InputValue key="placeholder" {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue key="value" {id} input={configuration.defaultValue} bind:value={defaultValue} />

<InitializeComponent {id} />
{#if render}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="editor-wrapper h-full flex-col flex max-h-full overflow-hidden"
		onpointerdown={stopPropagation(() => {
			if (!$connectingInput.opened) {
				$selectedComponent = [id]
			}
		})}
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
