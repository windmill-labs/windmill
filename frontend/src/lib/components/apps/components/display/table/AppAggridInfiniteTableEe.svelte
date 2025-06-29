<script module>
</script>

<script lang="ts">
	import type { AppInput } from '$lib/components/apps/inputType'
	import type { ComponentCustomCSS, RichConfigurations } from '$lib/components/apps/types'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { Loader2 } from 'lucide-svelte'
	import type { TableAction } from '$lib/components/apps/editor/component'
	import AppAggridInfiniteTable from './AppAggridInfiniteTable.svelte'

	interface Props {
		id: string
		license: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		render: boolean
		customCss?: ComponentCustomCSS<'aggridinfinitecomponentee'> | undefined
		actions?: TableAction[]
	}

	let {
		id,
		license,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		render,
		customCss = undefined,
		actions = []
	}: Props = $props()

	let loaded = $state(false)
	async function load() {
		await import('ag-grid-enterprise')
		const { LicenseManager } = await import('ag-grid-enterprise')
		LicenseManager.setLicenseKey(license)

		loaded = true
	}

	load()
</script>

{#if loaded}
	<AppAggridInfiniteTable
		{id}
		{componentInput}
		{configuration}
		{initializing}
		{render}
		{customCss}
		{actions}
	/>
{:else}
	<Loader2 class="animate-spin" />
{/if}
