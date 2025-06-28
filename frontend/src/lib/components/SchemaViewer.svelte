<script lang="ts">
	import type { Schema } from '$lib/common'
	import { copyToClipboard, emptySchema } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import TableCustom from './TableCustom.svelte'
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
		<Tab value="arguments">Arguments</Tab>
		<Tab value="advanced">Advanced</Tab>
		{#snippet content()}
			<div class="overflow-auto pt-2">
				<TabContent value="arguments">
					{#if schema && schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
						<div class="flex flex-row">
							<TableCustom>
								<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
								<tr slot="header-row" class="underline">
									<th>name</th>
									<th>type</th>
									<th>description</th>
									<th>default</th>
									<th>format</th>
									<th>required</th>
								</tr>
								{#snippet body()}
									<tbody>
										{#each getProperties(schema) as [name, property] (name)}
											<tr>
												<td class="font-semibold pl-1">{name}</td>
												<td
													><Badge color="blue"
														>{#if !property.type}
															any
														{:else}
															{property.type}
														{/if}</Badge
													></td
												>
												<td>{property.description ?? ''}</td>
												<td
													>{property.default == '<function call>'
														? '<function call>'
														: property.default
															? JSON.stringify(property.default)
															: ''}</td
												>
												<td
													>{property.format ?? ''}
													{property.contentEncoding
														? `(encoding: ${property.contentEncoding})`
														: ''}</td
												>
												<td
													>{#if schema.required.includes(name)}
														<span class="text-red-600 font-bold text-lg">*</span>
													{/if}</td
												>
											</tr>
										{/each}
									</tbody>
								{/snippet}
							</TableCustom>
						</div>
					{:else}
						<div class="text-secondary text-xs italic m-1">No arguments</div>
					{/if}
				</TabContent>
				<TabContent value="advanced">
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
