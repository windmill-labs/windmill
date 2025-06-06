<script lang="ts">
	import '@codingame/monaco-vscode-standalone-json-language-features'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import TriggerableByAI from './TriggerableByAI.svelte'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined

	export let code: string | undefined
	export let value: any = undefined
	export let error = ''
	export let editor: SimpleEditor | undefined = undefined
	export let small = false
	export let loadAsync = false

	$: tooBig = code && code?.length > 1000000

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function parseJson() {
		try {
			console.log('parseJson', code)
			if (code == '') {
				value = undefined
				error = ''
				return
			}
			value = JSON.parse(code ?? '')
			dispatchIfMounted('changeValue', value)
			error = ''
		} catch (e) {
			error = e.message
		}
	}
	$: code != undefined && parseJson()
	$: console.log('code', code)
</script>

{#if tooBig}
	<span class="text-tertiary">JSON to edit is too big</span>
{:else}
	<TriggerableByAI
		id={'json-editor'}
		description={'JSON editor'}
		onTrigger={(newValue) => {
			code = newValue
			if (editor && typeof newValue === 'string') {
				editor.setCode(newValue)
			}
			parseJson()
		}}
	/>
	<div class="flex flex-col w-full">
		<div class="border w-full">
			<SimpleEditor
				{loadAsync}
				{small}
				on:focus
				on:blur
				bind:this={editor}
				on:change
				autoHeight
				lang="json"
				bind:code
				class={$$props.class}
			/>
		</div>
		{#if error != ''}
			<span class="text-red-600 text-xs">{error}</span>
		{/if}
	</div>
{/if}
