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
	import type { CustomComponentConfig } from '../../editor/component'
	import { Loader2 } from 'lucide-svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let render: boolean
	export let componentInput: AppInput | undefined
	export let customComponent: CustomComponentConfig

	let divId = `custom-component-${id}`
	const { worldStore, workspace } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		output: undefined,
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
			outputs.output.set(output)
		}
	}
	let loaded = false
	let renderer: ((props: CCProps<number>) => void) | undefined = undefined
	// $: renderer && divEl && renderer(ccProps)
	onMount(async () => {
		// //@ts-ignore
		// await import('http://localhost:3000/app.iife.js')
		/* @vite-ignore */

		if (customComponent?.additionalLibs?.reactVersion) {
			let reactVersion = customComponent.additionalLibs.reactVersion
			//@ts-ignore
			await import(
				/* @vite-ignore */
				/* webpackIgnore: true */
				`https://unpkg.com/react@${reactVersion}/umd/react.production.min.js`
			)

			//@ts-ignore
			await import(
				/* @vite-ignore */
				/* webpackIgnore: true */
				`https://unpkg.com/react-dom@${reactVersion}/umd/react-dom.production.min.js`
			)
		}
		//@ts-ignore
		await import(
			/* @vite-ignore */
			/* webpackIgnore: true */
			`/api/w/${workspace ?? 'NO_W'}/resources_u/custom_component/${customComponent.name}`
		)
		loaded = true
		try {
			renderer = globalThis.windmill[customComponent?.name ?? 'no_name']
			if (!renderer) {
				sendUserToast(
					'Custom Component seem to be ill-defined (renderer missing). is COMPONENT_NAME in vite.config.ts matching the name of the custom component?',
					true
				)
				return
			}
			renderer?.(ccProps)
		} catch (e) {
			sendUserToast('Custom Component seem to be ill-defined', true)
			console.error(e)
		}
		console.log('mounted', render, setRender)
	})
	let result

	$: render != undefined && setRender && handleRender()
	function handleRender() {
		setRender?.(render)
	}

	$: result != undefined && setInput && handleResult()
	function handleResult() {
		setInput?.(result)
	}
	// $: result && setInput && setInput(result)
</script>

<InitializeComponent {id} />
<div class="w-full h-full overflow-auto {customComponent?.name ?? 'no_name'}">
	<RunnableWrapper {outputs} render autoRefresh {componentInput} {id} bind:result>
		{#if !loaded}
			<Loader2 class="animate-spin" />
		{/if}
		<div id={divId}></div>
	</RunnableWrapper>
</div>
