<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'

	import Select from './select/Select.svelte'
	import ExploreAssetButton, { assetCanBeExplored } from './ExploreAssetButton.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'

	interface Props {
		value?: string | undefined
		disabled?: boolean
		disablePortal?: boolean
		showSchemaExplorer?: boolean
		placeholder?: string | undefined
		selectInputClass?: string
		class?: string
		onClear?: () => void
	}

	let {
		value = $bindable(undefined),
		disabled = false,
		disablePortal = false,
		showSchemaExplorer = false,
		placeholder = undefined,
		selectInputClass = '',
		class: className = '',
		onClear = undefined
	}: Props = $props()

	let datatables = usePromise(() =>
		WorkspaceService.listDataTables({ workspace: $workspaceStore ?? '' })
	)
	let dbManagerDrawer = $derived(globalDbManagerDrawer.val)
</script>

<div class={className}>
	<Select
		items={datatables.value?.map((d) => ({ value: d })) ?? []}
		bind:value
		loading={datatables.status === 'loading'}
		{disabled}
		{disablePortal}
		{placeholder}
		inputClass={selectInputClass}
		{onClear}
	/>
	{#if showSchemaExplorer && value && assetCanBeExplored({ kind: 'datatable', path: value })}
		<ExploreAssetButton
			class="mt-1 w-fit"
			asset={{ kind: 'datatable', path: value }}
			{dbManagerDrawer}
		/>
	{/if}
</div>
