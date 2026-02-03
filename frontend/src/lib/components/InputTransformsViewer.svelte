<script lang="ts">
	import type { InputTransform } from '$lib/gen'
	import { cleanExpr } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import Row from './table/Row.svelte'

	export let inputTransforms: Record<string, InputTransform>
	$: entries = Object.entries(inputTransforms)
</script>

{#if entries.length}
	<DataTable size="xs" containerClass="bg-surface-tertiary">
		<Head>
			<tr class="w-full">
				<Cell head first>Input</Cell>
				<Cell head last>Value</Cell>
			</tr>
		</Head>

		<tbody class="divide-y w-full">
			{#each entries as [key, val]}
				<Row>
					<Cell first>{key}</Cell>
					<Cell last>
						{#if val.type == 'static'}
							{#if typeof val.value == 'object'}
								<ObjectViewer json={val.value} />
							{:else}
								<span class="text-xs text-primary whitespace-pre-wrap font-mono">
									{val.value}
								</span>
							{/if}
						{:else if val.type == 'javascript'}
							<span class="text-xs text-primary whitespace-pre-wrap font-mono">
								{cleanExpr(val.expr)}
							</span>
						{/if}
					</Cell>
				</Row>
			{/each}
		</tbody>
	</DataTable>
{:else}
	<div class="text-primary text-xs"> No inputs </div>
{/if}
