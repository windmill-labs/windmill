<script lang="ts">
	import '@codingame/monaco-vscode-standalone-json-language-features'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import Button from './common/button/Button.svelte'

	export let code: string | undefined
	export let value: any = undefined
	export let error = ''
	export let editor: SimpleEditor | undefined = undefined
	export let small = false
	export let loadAsync = false

	$: tooBig = code && code?.length > 1000000
	let loadTooBigAnyway = false

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function parseJson() {
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

{#if tooBig && !loadTooBigAnyway}
	<div class="flex-1 text-sm">
		JSON is too big
		<Button size="xs2" variant="border" on:click={() => (loadTooBigAnyway = true)}>
			Load anyway
		</Button>
	</div>
{:else}
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
