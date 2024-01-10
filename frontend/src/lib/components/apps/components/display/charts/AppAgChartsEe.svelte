<script context="module">
</script>

<script lang="ts">
	import AppAgCharts from './AppAgCharts.svelte'
	import type { AppInput } from '$lib/components/apps/inputType'
	import type {
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '$lib/components/apps/types'
	import { Loader2 } from 'lucide-svelte'

	export let id: string
	export let license: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let datasets: RichConfiguration | undefined
	export let xData: RichConfiguration | undefined

	let loaded = false

	async function load() {
		await import('ag-charts-enterprise')

		const { AgCharts } = await import('ag-charts-enterprise')
		AgCharts.setLicenseKey(license)

		loaded = true
	}

	load()
</script>

{#if loaded}
	<AppAgCharts
		{id}
		{componentInput}
		{configuration}
		bind:initializing
		{render}
		{customCss}
		{datasets}
		{xData}
	/>
{:else}
	<Loader2 class="animate-spin" />
{/if}
