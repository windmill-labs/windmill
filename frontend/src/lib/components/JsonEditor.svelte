<script lang="ts">
	import '@codingame/monaco-vscode-standalone-json-language-features'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import Button from './common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import { inputBorderClass } from './text_input/TextInput.svelte'

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
		error = $bindable(),
		editor = $bindable(undefined),
		small = true,
		loadAsync = false,
		class: clazz = undefined,
		disabled = false,
		fixedOverflowWidgets = true
	}: Props = $props()

	let tooBig = $derived(code && code?.length > 1000000)
	let loadTooBigAnyway = $state(false)
	let focused = $state(false)

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
	$effect(() => {
		code != undefined && untrack(() => parseJson())
	})
</script>

{#if tooBig && !loadTooBigAnyway}
	<div class="flex-1 text-sm">
		JSON is too big
		<Button size="xs2" variant="default" on:click={() => (loadTooBigAnyway = true)}>
			Load anyway
		</Button>
	</div>
{:else}
	<div class="flex flex-col w-full">
		<div
			class={twMerge(
				'w-full rounded-md overflow-auto',
				inputBorderClass({ error: !!error, forceFocus: focused })
			)}
		>
			<SimpleEditor
				{loadAsync}
				{small}
				on:focus={() => (dispatch('focus'), (focused = true))}
				on:blur={() => (dispatch('blur'), (focused = false))}
				bind:this={editor}
				on:change
				autoHeight
				lang="json"
				bind:code
				class={clazz}
				{disabled}
				{fixedOverflowWidgets}
				renderLineHighlight="none"
			/>
		</div>
		{#if error != ''}
			<span class="text-red-600 text-xs mt-1">{error}</span>
		{/if}
	</div>
{/if}
