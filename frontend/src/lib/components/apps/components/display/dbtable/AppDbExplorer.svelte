<script lang="ts">
	import type { ComponentCustomCSS, RichConfigurations } from '../../../types'
	import type { AppInput, ResultInput, RunnableByName } from '../../../inputType'
	import { Preview } from '$lib/gen'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { initConfig } from '$lib/components/apps/editor/appUtils'
	import AppInputs from '$lib/components/apps/editor/AppInputs.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	let resolvedConfig = initConfig(
		components['dbexplorercomponent'].initialData.configuration,
		configuration
	)

	resolvedConfig.type.configuration.postgresql

	function makeQuery(
		table: string | undefined,
		columns: string[],
		pageSize: number | undefined,
		page: number
	) {
		if (!table) throw new Error('Table name is required')

		if (!pageSize) {
			return `SELECT ${columns ? columns.join(', ') : '*'} FROM ${table}`
		}

		return `SELECT ${columns ? columns.join(', ') : '*'} FROM ${table} LIMIT ${pageSize} OFFSET ${
			pageSize * page
		}`
	}

	function createResultInput(): AppInput {
		const runnable: RunnableByName = {
			name: 'AppDbExplorer',
			type: 'runnableByName',
			inlineScript: {
				content: makeQuery(
					resolvedConfig.type.configuration.postgresql.table,
					resolvedConfig.type.configuration.postgresql.columns,
					resolvedConfig.type.configuration.postgresql.pageSize,
					1
				),
				language: Preview.language.POSTGRESQL,
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {
						database: {
							description: 'Database name',
							type: 'object',
							format: 'resource-postgresql'
						}
					},
					required: ['database'],
					type: 'object'
				}
			}
		}
		return {
			runnable,
			fields: {
				database: {
					type: 'static',
					value: '$res:u/faton/truthful_postgresql',
					fieldType: 'object',
					format: 'resource-postgresql'
				}
			},
			type: 'runnable'
		}
	}
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

<span>db explorer</span>
