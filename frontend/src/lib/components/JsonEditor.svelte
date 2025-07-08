<script lang="ts">
	import '@codingame/monaco-vscode-standalone-json-language-features'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

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
		if (tooBig) return
		try {
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
</script>

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
			bind:code={() => (tooBig ? '// JSON is too big to edit' : code), (c) => (code = c)}
			class={$$props.class}
		/>
	</div>
	{#if error != ''}
		<span class="text-red-600 text-xs">{error}</span>
	{/if}
</div>
