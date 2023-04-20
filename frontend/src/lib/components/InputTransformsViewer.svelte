<script lang="ts">
	import type { InputTransform } from '$lib/gen'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { cleanExpr } from './flows/utils'

	export let inputTransforms: Record<string, InputTransform>
	$: entries = Object.entries(inputTransforms)
</script>

{#if entries.length}
	<ul class="mb-1">
		{#each entries as [key, val]}
			<li class="flex pb-2 last:pb-0">
				<span class="font-black text-gray-700 text-xs">{key}:</span>
				{#if val.type == 'static'}
					<ObjectViewer json={val.value} />
				{:else}
					<span class="text-xs text-black whitespace-pre-wrap ml-2">
						{cleanExpr(val.expr)}
					</span>
				{/if}
			</li>
		{/each}
	</ul>
{:else}
	<div class="text-gray-600 text-sm"> No inputs </div>
{/if}
