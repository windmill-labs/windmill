<script lang="ts">
	import type { Schema } from '$lib/common'
	import { emptySchema } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import github from 'svelte-highlight/styles/github'
	import TableCustom from './TableCustom.svelte'

	export let schema: Schema | undefined = emptySchema()

	let viewJsonSchema = false
</script>

<svelte:head>
	{@html github}
</svelte:head>

<div class="w-full">
	<div class="flex flex-col sm:flex-row text-base">
		<button
			class="text-xs sm:text-base py-1 px-6 block hover:text-blue-500 focus:outline-noneborder-gray-200  {viewJsonSchema
				? 'text-gray-500 '
				: 'text-gray-700 font-semibold  '}"
			on:click={() => (viewJsonSchema = false)}
		>
			arguments
		</button><button
			class="py-1 px-6 block hover:text-blue-500 focus:outline-none border-gray-200  {viewJsonSchema
				? 'text-gray-700 font-semibold '
				: 'text-gray-500'}"
			on:click={() => (viewJsonSchema = true)}
		>
			advanced
		</button>
	</div>
</div>
<!--json schema or table view-->
<div class="border-t py-1">
	<div class={viewJsonSchema ? 'hidden' : ''}>
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
								<td>{name}</td>
								<td
									>{#if !property.type} any {:else} {property.type} {/if}</td
								>
								<td>{property.description}</td>
								<td
									>{property.default == '<function call>'
										? '<function call>'
										: property.default
										? JSON.stringify(property.default)
										: ''}</td
								>
								<td>{property.format ?? ''}</td>
								<td>{schema.required.includes(name) ? 'required' : 'optional'}</td>
							</tr>
						{/each}
					</tbody>
				</TableCustom>
			</div>
		{:else}
			<div class="text-gray-700 text-xs italic">This script has no arguments</div>
		{/if}
	</div>
	<div class={viewJsonSchema ? '' : 'hidden'}>
		<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
	</div>
</div>
