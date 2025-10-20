<script lang="ts">
	import type { InputTransform } from '$lib/gen'
	import { cleanExpr } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'

	export let inputTransforms: Record<string, InputTransform>
	$: entries = Object.entries(inputTransforms)
</script>

{#if entries.length}
	<ul class="mb-1">
		{#each entries as [key, val]}
			<li class="flex pb-2 last:pb-0">
				<span class="font-black text-secondary text-xs">{key}:</span>
				{#if val.type == 'static'}
					{#if typeof val.value == 'object'}
						<ObjectViewer json={val.value} />
					{:else}
						<span class="text-xs text-primary whitespace-pre-wrap ml-2">
							{val.value}
						</span>
					{/if}
				{:else}
					<span class="text-xs text-primary whitespace-pre-wrap ml-2">
						{cleanExpr(val.expr)}
					</span>
				{/if}
			</li>
		{/each}
	</ul>
{:else}
	<div class="text-primary text-xs"> No inputs </div>
{/if}
