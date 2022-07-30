<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import python from 'svelte-highlight/languages/python'
	import Modal from '../Modal.svelte'
	import { createScriptFromInlineScript, fork, removeModule } from './flowStore'
	import { scrollIntoView } from './utils'

	export let open: number
	export let i: number
	export let shouldPick = false
	export let mod: FlowModule

	let modalViewer: Modal
	let modalViewerContent = ''
	let modalViewerLanguage: 'deno' | 'python3' = 'deno'

	async function viewCode() {
		if (mod.value.type == 'script') {
			const { content, language } = await getScriptByPath(mod.value.path!)
			modalViewerContent = content
			modalViewerLanguage = language
			modalViewer.openModal()
		} else {
			throw Error('Not a script')
		}
	}

	function scrollTo({ target }) {
		const el = document.querySelector(target.getAttribute('href'))
		scrollIntoView(el)
	}
</script>

<div class="flex flex-row w-full space-x-2">
	<slot />
	<a href="#module-{i}" class="grow" on:click={() => (open = i)} on:click|preventDefault={scrollTo}
		><div class="w-full" /></a
	>

	<div class="flex flex-row space-x-2">
		{#if mod.value.type === 'script' && !shouldPick}
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

		{#if mod.value.type === 'rawscript' && !shouldPick}
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
	<div slot="title">Script {'path' in mod?.value ? mod?.value.path : ''}</div>
	<div slot="content">
		{#if modalViewerLanguage === 'python3'}
			<Highlight language={python} code={modalViewerContent} />
		{:else if modalViewerLanguage === 'deno'}
			<Highlight language={typescript} code={modalViewerContent} />
		{/if}
	</div>
</Modal>
