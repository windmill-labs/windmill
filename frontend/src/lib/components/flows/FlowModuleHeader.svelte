<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import Modal from '../Modal.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { scrollIntoView } from './utils'

	export let indexes: number[]
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
		const element = document.querySelector(target.getAttribute('href'))
		scrollIntoView(element)
	}

	const dispatch = createEventDispatcher()
</script>

<a href="#module-{indexes.join('-')}" class="grow text-inherit" on:click|preventDefault={scrollTo}>
	<h3 class="text-sm font-bold text-gray-900">
		{#if 'path' in mod.value && mod.value.path}
			{mod.value.path}
		{:else if 'language' in mod.value && mod.value.language}
			Inline {mod.value.language}
		{:else}
			Select a script
		{/if}
	</h3>
</a>

<div class="flex flex-row space-x-2">
	{#if mod.value.type === 'script' && !shouldPick}
		<Button size="xs" color="alternative" on:click={() => dispatch('fork')}>
			<Icon data={faCodeBranch} class="mr-2" />
			Fork
		</Button>
		<Button size="xs" color="alternative" on:click={viewCode}>
			<Icon data={faCode} class="mr-2" />
			View code
		</Button>
	{/if}

	{#if mod.value.type === 'rawscript' && !shouldPick}
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
		Remove step
	</Button>
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
