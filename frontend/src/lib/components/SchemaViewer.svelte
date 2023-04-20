<script lang="ts">
	import type { Schema } from '$lib/common'
	import { emptySchema } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import TableCustom from './TableCustom.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import { Badge } from './common'

	export let schema: Schema | undefined = emptySchema()
</script>

<div class="w-full">
	<Tabs selected="arguments">
		<Tab value="arguments">Arguments</Tab>
		<Tab value="advanced">Advanced</Tab>
		<svelte:fragment slot="content">
			<div class="overflow-auto pt-2">
				<TabContent value="arguments">
					{#if schema && schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
						<div class="flex flex-row">
							<TableCustom>
								<tr slot="header-row" class="underline">
									<th>name</th>
									<th>type</th>
									<th>description</th>
									<th>default</th>
									<th>format</th>
									<th>required</th>
								</tr>
								<tbody slot="body">
									{#each Object.entries(schema.properties) as [name, property] (name)}
										<tr>
											<td class="font-semibold pl-1">{name}</td>
											<td
												><Badge color="blue"
													>{#if !property.type} any {:else} {property.type} {/if}</Badge
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
							</TableCustom>
						</div>
					{:else}
						<div class="text-gray-700 text-xs italic m-1">No arguments</div>
					{/if}
				</TabContent>
				<TabContent value="advanced">
					<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
				</TabContent>
			</div>
		</svelte:fragment>
	</Tabs>
</div>
