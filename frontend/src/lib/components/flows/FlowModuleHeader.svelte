<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath, valueToTsType } from '$lib/utils'
	import {
		faArrowDown,
		faClose,
		faCode,
		faCodeBranch,
		faSave,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import IconedPath from '../IconedPath.svelte'
	import IconedResourceType from '../IconedResourceType.svelte'
	import Modal from '../Modal.svelte'
	import { isEmptyFlowModule } from './flowStateUtils'
	import { stepOpened } from './stepOpenedStore'

	export let indexes: number[]
	export let mod: FlowModule

	$: shouldPick = isEmptyFlowModule(mod)

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

	const dispatch = createEventDispatcher()
	$: opened = $stepOpened === String(indexes.join('-'))
</script>

<div on:click={() => stepOpened.update(() => String(indexes.join('-')))}>
	<h3 class="text-sm font-bold text-gray-900">
		{#if 'path' in mod.value && mod.value.path}
			<IconedPath path={mod.value.path} />
		{:else if 'language' in mod.value && mod.value.language}
			Inline {mod.value.language}
		{:else}
			Select a script
		{/if}
	</h3>
</div>

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
	{#if opened}
		<Button size="xs" color="dark" on:click={() => stepOpened.update(() => undefined)}>
			<Icon data={faClose} />
		</Button>
	{:else}
		<Button
			size="xs"
			color="light"
			on:click={() => stepOpened.update(() => String(indexes.join('-')))}
		>
			<Icon data={faArrowDown} />
		</Button>
	{/if}
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
