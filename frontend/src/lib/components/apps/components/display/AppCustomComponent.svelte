<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, RichConfigurations } from '../../types'

	type CCProps<Init, Output> = {
		initState: Init | undefined
		id: string
		saveInitState: (state: Init) => void
		setOutput: (output: Output) => void
	}

	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { AppInput } from '../../inputType'
	import { RunnableWrapper } from '../helpers'

	export let id: string
	export let configuration: RichConfigurations
	export let render: boolean
	export let componentInput: AppInput | undefined

	let divId = 'custom-component-{id}'
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let divEl: HTMLDivElement
	const outputs = initOutput($worldStore, id, {
		result: undefined,
		ccOutput: undefined,
		loading: false
	})

	let ccProps: CCProps<any, any> = {
		initState: undefined,
		id: divId,
		saveInitState: (state) => {
			console.log('saveInitState', state)
		},
		setOutput: (output) => {
			outputs.ccOutput.set(output)
			console.log('setOutput', output)
		}
	}

	let renderer: (props: CCProps<any, any>) => void

	// $: renderer && divEl && renderer(ccProps)

	o{
		// //@ts-ignore
		// await import('http://localhost:3000/app.iife.js')
		/* @vite-ignore */

		//@ts-ignore
		await import('https://unpkg.com/react@18.2.0/umd/react.development.js')

		//@ts-ignore
		await import('https://unpkg.com/react-dom@18.2.0/umd/react-dom.development.js')
		//@ts-ignore
		await import('http://localhost:3000/cc.iife.js')

		try {
			renderer = globalThis.windmill['foo']
			renderer(ccProps)
		} catch (e) {
			sendUserToast('Custom Component seem to be ill-defined', true)
			console.error(e)
		}
	})
</script>

<InitializeComponent {id} />

{#if render}
	<RunnableWrapper
		{outputs}
		{render}
		autoRefresh
		{componentInput}
		{id}
		bind:result={ccProps.initState}
	>
		<div bind:this={divEl} id={divId} />
	</RunnableWrapper>
{/if}
