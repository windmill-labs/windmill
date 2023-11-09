<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext } from '../../types'

	type CCProps<Input> = {
		id: string
		render: boolean
		passSetters: (setter: Setter<Input>) => void
		setOutput: (output: any) => void
	}

	interface Setter<Input> {
		onRender(bool: boolean): void
		onInput(input: Input): void
	}

	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { AppInput } from '../../inputType'
	import { RunnableWrapper } from '../helpers'
	import { workspaceStore } from '$lib/stores'
	import type { CustomComponentConfig } from '../../editor/component'

	export let id: string
	export let render: boolean
	export let componentInput: AppInput | undefined
	export let customComponent: CustomComponentConfig

	let divId = `custom-component-${id}`
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		ccOutput: undefined,
		loading: false
	})

	let setInput
	let setRender
	let ccProps: CCProps<any> = {
		render,
		id: divId,
		passSetters: (setter) => {
			setInput = setter.onInput
			setRender = setter.onRender
		},
		setOutput: (output) => {
			outputs.ccOutput.set(output)
			console.log('setOutput', output)
		}
	}
	let loaded = false

	// $: renderer && divEl && renderer(ccProps)
	onMount(async () => {
		// //@ts-ignore
		// await import('http://localhost:3000/app.iife.js')
		/* @vite-ignore */

		if (customComponent?.additionalLibs?.reactVersion) {
			let reactVersion = customComponent.additionalLibs.reactVersion
			//@ts-ignore
			await import(`https://unpkg.com/react@${reactVersion}/umd/react.development.js`)

			//@ts-ignore
			await import(`https://unpkg.com/react-dom@${reactVersion}/umd/react-dom.development.js`)
		}
		//@ts-ignore
		await import(
			`http://localhost:3000/api/w/${$workspaceStore ?? 'NO_W'}/resources/custom_component/${
				customComponent.name
			}`
		)
		loaded = true
		try {
			let renderer: (props: CCProps<number>) => void = globalThis.windmill[customComponent.name]
			renderer?.(ccProps)
		} catch (e) {
			sendUserToast('Custom Component seem to be ill-defined', true)
			console.error(e)
		}
	})
	let result

	$: render != undefined && handleRender()
	function handleRender() {
		setRender?.(render)
	}
	$: result != undefined && handleResult()
	function handleResult() {
		setInput?.(result)
	}
	// $: result && setInput && setInput(result)
</script>

<InitializeComponent {id} />

{#if render}
	<div class="w-full h-full overflow-auto {customComponent.name}">
		<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:result>
			<div id={divId} />
		</RunnableWrapper>
	</div>
{/if}
