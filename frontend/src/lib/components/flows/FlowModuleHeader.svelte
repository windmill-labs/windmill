<script lang="ts">
	import { FlowModuleValue, Script, type FlowModule } from '$lib/gen'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import { python, typescript } from 'svelte-highlight/languages'
	import github from 'svelte-highlight/styles/github'
	import Modal from '../Modal.svelte'
	import { createScriptFromInlineScript, fork, removeModule } from './flowStore'
	import { getScriptByPath } from './utils'

	export let open: number
	export let i: number
	export let shouldPick = false
	export let mod: FlowModule

	let modalViewer: Modal
	let modalViewerContent = ''
	let modalViewerLanguage: Script.language = Script.language.DENO

	async function viewCode() {
		const { content, language } = await getScriptByPath(mod.value.path!)
		modalViewerContent = content
		modalViewerLanguage = language
		modalViewer.openModal()
	}
</script>

<svelte:head>
	{@html github}
</svelte:head>

<div>
	<slot />
</div>
<div>
	{#if mod.value.type === FlowModuleValue.type.SCRIPT && !shouldPick}
		<button
			type="button"
			on:click={() => {
				open = i
				fork(i)
			}}
			class="default-secondary-button-v2 text-xs"
		>
			<Icon data={faCodeBranch} class={`w-4 mr-2 h-4`} />
			Fork
		</button>
		<button type="button" on:click={viewCode} class="default-secondary-button-v2 text-xs">
			<Icon data={faCode} class="w-4 mr-2 h-4" />
			View code
		</button>
	{/if}

	<div class="flex flex-row space-x-2">
		{#if mod.value.type === FlowModuleValue.type.RAWSCRIPT && !shouldPick}
			<button
				type="button"
				on:click={() => createScriptFromInlineScript(i)}
				class="default-secondary-button-v2 text-sm"
			>
				<Icon data={faSave} class="w-4 mr-4" />
				Save to workspace
			</button>
		{/if}
		<button
			type="button"
			on:click={() => {
				open = -1
				removeModule(i)
			}}
			class="default-secondary-button-v2 text-xs"
		>
			<Icon data={faTrashAlt} class="w-4 mr-2 h-4" />
			Remove step
		</button>
	</div>
</div>

<Modal bind:this={modalViewer}>
	<div slot="title">Script {mod?.value.path}</div>
	<div slot="content">
		{#if modalViewerLanguage === 'python3'}
			<Highlight language={python} code={modalViewerContent} />
		{:else if modalViewerLanguage === 'deno'}
			<Highlight language={typescript} code={modalViewerContent} />
		{/if}
	</div>
</Modal>
