<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Modal from '$lib/components/Modal.svelte'

	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowModuleWidthContext } from './FlowModule.svelte'

	export let module: FlowModule

	$: shouldPick = isEmptyFlowModule(module)

	let modalViewer: Modal
	let modalViewerContent = ''
	let modalViewerLanguage: 'deno' | 'python3' | 'go' = 'deno'

	async function viewCode() {
		if (module.value.type == 'script') {
			const { content, language } = await getScriptByPath(module.value.path!)
			modalViewerContent = content
			modalViewerLanguage = language
			modalViewer.openModal()
		} else {
			throw Error('Not a script')
		}
	}

	const dispatch = createEventDispatcher()
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const { width, threshold } = getContext<FlowModuleWidthContext>('FlowModuleWidth')
	$: iconOnly = $width < threshold
</script>

<div class="flex flex-row space-x-2">
	{#if module.value.type === 'script' && !shouldPick}
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={() => dispatch('fork')}
			startIcon={{ icon: faCodeBranch }}
			{iconOnly}
		>
			Fork
		</Button>
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={viewCode}
			startIcon={{ icon: faCode }}
			{iconOnly}
		>
			View code
		</Button>
	{/if}

	{#if module.value.type === 'rawscript' && !shouldPick}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faSave }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			{iconOnly}
		>
			Save to workspace
		</Button>
	{/if}
	<Button
		size="xs"
		color="light"
		variant="border"
		on:click={() => dispatch('delete')}
		startIcon={{ icon: faTrashAlt }}
		{iconOnly}
	>
		{$selectedId.includes('failure') ? 'Delete error handler' : 'Remove step'}
	</Button>
</div>

<Modal bind:this={modalViewer}>
	<div slot="title">Script {'path' in module?.value ? module?.value.path : ''}</div>
	<div slot="content">
		<HighlightCode language={modalViewerLanguage} code={modalViewerContent} />
	</div>
</Modal>
