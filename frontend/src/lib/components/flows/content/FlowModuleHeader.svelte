<script lang="ts">
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Modal from '$lib/components/Modal.svelte'

	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { FlowEditorContext } from '../types'

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
</script>

<div class="flex flex-row space-x-2" on:click|stopPropagation={() => undefined}>
	{#if module.value.type === 'script' && !shouldPick}
		<Button size="xs" color="alternative" on:click={() => dispatch('fork')}>
			<Icon data={faCodeBranch} class="mr-2" />
			Fork
		</Button>
		<Button size="xs" color="alternative" on:click={viewCode}>
			<Icon data={faCode} class="mr-2" />
			View code
		</Button>
	{/if}

	{#if module.value.type === 'rawscript' && !shouldPick}
		<Button size="xs" color="alternative" on:click={() => dispatch('createScriptFromInlineScript')}>
			<Icon data={faSave} class="mr-2" />
			Save to workspace
		</Button>
	{/if}
	<Button
		size="xs"
		color="alternative"
		on:click={() => {
			dispatch('delete')
		}}
	>
		<Icon data={faTrashAlt} class="mr-2" />
		{$selectedId.includes('failure') ? 'Delete failure module' : 'Remove step'}
	</Button>
</div>

<Modal bind:this={modalViewer}>
	<div slot="title">Script {'path' in module?.value ? module?.value.path : ''}</div>
	<div slot="content">
		<HighlightCode language={modalViewerLanguage} code={modalViewerContent} />
	</div>
</Modal>
