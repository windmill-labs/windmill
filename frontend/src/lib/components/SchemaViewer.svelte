<script lang="ts">
	import type { Schema } from '$lib/common'
	import { copyToClipboard, emptySchema } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import Row from './table/Row.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import { Badge } from './common'
	import HighlightTheme from './HighlightTheme.svelte'
	import Button from './common/button/Button.svelte'
	import { ClipboardCopy } from 'lucide-svelte'

	interface Props {
		schema?: Schema | undefined | any
	}

	let { schema = emptySchema() }: Props = $props()

	function getProperties(schema: Schema) {
		if (schema.properties) {
			return Object.entries(schema.properties)
		}
		return []
	}
</script>

<HighlightTheme />

<div class="w-full">
	<Tabs selected="arguments">
		<Tab value="arguments" label="Inputs" />
		<Tab value="json" label="JSON" />
		{#snippet content()}
			<div class="overflow-auto pt-2">
				<TabContent value="arguments">
					{#if schema && schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
						<DataTable size="sm" containerClass="bg-surface-tertiary">
							<Head>
								<tr class="w-full">
									<Cell head first>Name</Cell>
									<Cell head>Type</Cell>
									<Cell head>Description</Cell>
									<Cell head>Default</Cell>
									<Cell head>Format</Cell>
									<Cell head last>Required</Cell>
								</tr>
							</Head>

							<tbody class="divide-y w-full">
								{#each getProperties(schema) as [name, property] (name)}
									<Row>
										<Cell first>
											<span class="font-semibold">{name}</span>
										</Cell>
										<Cell>
											<Badge color="blue">
												{#if !property.type}
													any
												{:else}
													{property.type}
												{/if}
											</Badge>
										</Cell>
										<Cell wrap>
											{property.description ?? ''}
										</Cell>
										<Cell>
											{property.default == '<function call>'
												? '<function call>'
												: property.default
													? JSON.stringify(property.default)
													: ''}
										</Cell>
										<Cell>
											{property.format ?? ''}
											{property.contentEncoding ? `(encoding: ${property.contentEncoding})` : ''}
										</Cell>
										<Cell last>
											{#if schema.required.includes(name)}
												<span class="text-red-600 font-bold text-lg">*</span>
											{/if}
										</Cell>
									</Row>
								{/each}
							</tbody>
						</DataTable>
					{:else}
						<div class="text-secondary text-xs italic m-1">No arguments</div>
					{/if}
				</TabContent>
				<TabContent value="json">
					<div class="h-full relative">
						<Button
							wrapperClasses="absolute top-2 right-2 z-20"
							on:click={() => copyToClipboard(JSON.stringify(schema, null, 4))}
							color="light"
							size="xs2"
							startIcon={{
								icon: ClipboardCopy
							}}
							iconOnly
						/>
						<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
					</div>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
