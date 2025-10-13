<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	import Select from './select/Select.svelte'
	import DbManagerDrawer from './DBManagerDrawer.svelte'
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

	let ducklakes = usePromise(() =>
		WorkspaceService.listDucklakes({ workspace: $workspaceStore ?? '' })
	)
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
</script>

<div class={className}>
	<Select
		items={ducklakes.value?.map((d) => ({ value: d })) ?? []}
		bind:value
		loading={ducklakes.status === 'loading'}
		{disabled}
		{disablePortal}
		{placeholder}
		inputClass={selectInputClass}
		{onClear}
	/>
	{#if showSchemaExplorer && value && assetCanBeExplored({ kind: 'ducklake', path: value })}
		<ExploreAssetButton
			class="mt-1 w-fit"
			asset={{ kind: 'ducklake', path: value }}
			{dbManagerDrawer}
		/>
	{/if}

	<DbManagerDrawer bind:this={dbManagerDrawer} />
</div>
