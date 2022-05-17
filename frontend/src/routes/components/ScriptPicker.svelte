<script lang="ts">
	import { sendUserToast } from '../../utils'
	import { ScriptService, FlowService } from '../../gen'

	import Icon from 'svelte-awesome'
	import { faSearch } from '@fortawesome/free-solid-svg-icons'
	import { workspaceStore } from '../../stores'
	import { createEventDispatcher } from 'svelte'
	import ItemPicker from './ItemPicker.svelte'
	import RadioButton from './RadioButton.svelte'
	import Modal from './Modal.svelte'
	import { Highlight } from 'svelte-highlight'
	import { python, typescript } from 'svelte-highlight/src/languages'

	import github from 'svelte-highlight/src/styles/github'

	export let scriptPath: string | undefined = undefined
	export let allowFlow = false
	export let isFlow = false

	let items: { summary: String; path: String; version?: String }[] = []
	let itemPicker: ItemPicker
	let modalViewer: Modal
	let code: string = ''
	let lang: 'deno' | 'python3' | undefined

	const dispatch = createEventDispatcher()

	async function getScript() {
		const script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path: scriptPath!
		})
		code = script.content
		lang = script.language
	}

	async function loadItems(isFlow: boolean): Promise<void> {
		try {
			if (isFlow) {
				items = await FlowService.listFlows({ workspace: $workspaceStore! })
			} else {
				items = await ScriptService.listScripts({ workspace: $workspaceStore! })
			}
		} catch (err) {
			sendUserToast(`Could not load items: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore) {
			loadItems(isFlow)
		}
	}
</script>

<svelte:head>
	{@html github}
</svelte:head>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		scriptPath = path
	}}
	itemName={isFlow ? 'Flow' : 'Script'}
	extraField="summary"
	loadItems={async () => {
		return items
	}}
/>

<div class="flex flex-row items-center">
	{#if allowFlow}
		<RadioButton
			bind:value={isFlow}
			options={[
				['Script', false],
				['Flow', true]
			]}
		/>
	{/if}
	<select
		bind:value={scriptPath}
		on:change={() => {
			dispatch('select', { path: scriptPath })
		}}
		class="max-w-lg"
	>
		<option value={undefined} />
		{#each items as s}
			<option value={s.path}>{s.path} {s.summary ? ' | ' + s.summary : ''}</option>
		{/each}
	</select>
	<button on:click={() => itemPicker.openModal()}
		><Icon class="mx-4 text-gray-700 text-opacity-70" data={faSearch} /></button
	>
	{#if scriptPath != undefined && scriptPath != ''}
		<button
			class="text-xs text-blue-500"
			on:click={async () => {
				await getScript()
				modalViewer.openModal()
			}}>show code</button
		>
	{/if}
</div>

<Modal bind:this={modalViewer}>
	<div slot="title">Script {scriptPath}</div>
	<div slot="content">
		{#if lang == 'python3'}
			<Highlight language={python} {code} />
		{:else if lang == 'deno'}
			<Highlight language={typescript} {code} />
		{/if}
	</div>
</Modal>
