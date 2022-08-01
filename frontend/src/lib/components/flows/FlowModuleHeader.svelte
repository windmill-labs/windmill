<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'
	import { faCode, faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import github from 'svelte-highlight/styles/github'
	import Modal from '../Modal.svelte'
	import FlowBoxHeader from './FlowBoxHeader.svelte'
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

<svelte:head>
	{@html github}
</svelte:head>

<FlowBoxHeader>
	<a
		href="#module-{i}"
		class="grow text-inherit"
		on:click={() => (open = i)}
		on:click|preventDefault={scrollTo}
	>
		<slot />
	</a>

	<div class="flex flex-row space-x-2">
		{#if mod.value.type === 'script' && !shouldPick}
			<Button
				on:click={() => {
					open = i
					fork(i)
				}}
				size="sm"
				color="alternative"
			>
				<Icon data={faCodeBranch} class="mr-2" />
				Fork
			</Button>
			<Button size="sm" color="alternative" on:click={viewCode}>
				<Icon data={faCode} class="mr-2" />
				View code
			</Button>
		{/if}

		{#if mod.value.type === 'rawscript' && !shouldPick}
			<Button size="sm" color="alternative" on:click={() => createScriptFromInlineScript(i)}>
				<Icon data={faSave} class="mr-2" />
				Save to workspace
			</Button>
		{/if}
		<Button
			size="sm"
			color="alternative"
			on:click={() => {
				open = -1
				removeModule(i)
			}}
		>
			<Icon data={faTrashAlt} class="mr-2" />
			Remove step
		</Button>
	</div>
</FlowBoxHeader>

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
