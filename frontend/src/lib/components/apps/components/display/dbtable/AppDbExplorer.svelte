<script lang="ts">
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { initConfig } from '$lib/components/apps/editor/appUtils'
	import { createPostgresInput } from './utils'
	import AppAggridTable from '../table/AppAggridTable.svelte'
	import { getContext } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	const resolvedConfig = initConfig(
		components['dbexplorercomponent'].initialData.configuration,
		configuration
	)

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	$: input = createPostgresInput(
		resolvedConfig.type.configuration.postgresql.resource,
		resolvedConfig.type.configuration.postgresql.table,
		resolvedConfig.type.configuration.postgresql.columns,
		resolvedConfig.type.configuration.postgresql.pageSize,
		$worldStore.outputsById[id]?.page.peak() ?? 1
	)

	let updateCell: UpdateCell
</script>

{#each Object.keys(components['dbexplorercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<UpdateCell {id} bind:this={updateCell} />

<AppAggridTable
	{id}
	{configuration}
	bind:initializing
	componentInput={input}
	{customCss}
	{render}
	on:update={(e) => {
		const { row, column, value } = e.detail

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table,
			row,
			column,
			value
		)
	}}
/>
