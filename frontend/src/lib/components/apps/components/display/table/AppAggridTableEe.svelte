<script context="module">
</script>

<script lang="ts">
	import AppAggridTable from './AppAggridTable.svelte'
	import type { AppInput } from '$lib/components/apps/inputType'
	import type { ComponentCustomCSS, RichConfigurations } from '$lib/components/apps/types'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { Loader2 } from 'lucide-svelte'

	export let id: string
	export let license: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined

	let loaded = false
	async function load() {
		await import('ag-grid-enterprise')
		const { LicenseManager } = await import('ag-grid-enterprise')
		LicenseManager.setLicenseKey(license)

		loaded = true
	}

	load()
</script>

{#if loaded}
	<AppAggridTable {id} {componentInput} {configuration} {initializing} {render} {customCss} />
{:else}
	<Loader2 class="animate-spin" />
{/if}
