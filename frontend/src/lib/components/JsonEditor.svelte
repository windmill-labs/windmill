<script lang="ts">
	import { run } from 'svelte/legacy'

	import '@codingame/monaco-vscode-standalone-json-language-features'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import Button from './common/button/Button.svelte'

	interface Props {
		code: string | undefined
		value?: any
		error?: string
		editor?: SimpleEditor | undefined
		small?: boolean
		loadAsync?: boolean
		class?: string | undefined
		disabled?: boolean
		fixedOverflowWidgets?: boolean
	}

	let {
		code = $bindable(),
		value = $bindable(undefined),
		error = $bindable(''),
		editor = $bindable(undefined),
		small = false,
		loadAsync = false,
		class: clazz = undefined,
		disabled = false,
		fixedOverflowWidgets = true
	}: Props = $props()

	let tooBig = $derived(code && code?.length > 1000000)
	let loadTooBigAnyway = $state(false)

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
	run(() => {
		code != undefined && parseJson()
	})
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
				class={clazz}
				{disabled}
				{fixedOverflowWidgets}
			/>
		</div>
		{#if error != ''}
			<span class="text-red-600 text-xs">{error}</span>
		{/if}
	</div>
{/if}
