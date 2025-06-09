<script lang="ts">
	import { Popover } from '$lib/components/meltComponents'
	import { Loader2, X } from 'lucide-svelte'
	import { ContextIconMap, type ContextElement } from './context'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import { twMerge } from 'tailwind-merge'
	import {
		formatGraphqlSchema,
		formatSchema
	} from '$lib/components/apps/components/display/dbtable/utils'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { createEventDispatcher } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'

	export let contextElement: ContextElement
	export let deletable = false
	const icon = ContextIconMap[contextElement.type]
	let showDelete = false

	const dispatch = createEventDispatcher<{
		delete: void
	}>()
</script>

<Popover>
	<svelte:fragment slot="trigger">
		<div
			class={twMerge(
				'border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-tertiary text-xs cursor-default hover:bg-surface-hover hover:cursor-pointer'
			)}
			on:mouseenter={() => (showDelete = true)}
			on:mouseleave={() => (showDelete = false)}
			aria-label="Context element"
			role="button"
			tabindex={0}
		>
			<button on:click={() => dispatch('delete')} class:cursor-default={!deletable}>
				{#if showDelete && deletable}
					<X size={16} />
				{:else}
					<svelte:component this={icon} size={16} />
				{/if}
			</button>
			{contextElement.type === 'diff'
				? contextElement.title.replace(/_/g, ' ')
				: contextElement.title}
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		{#if contextElement.type === 'error'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<Highlight language={json} code={contextElement.content} class="w-full p-2" />
			</div>
		{:else if contextElement.type === 'db'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				{#if contextElement.schema && contextElement.schema.lang === 'graphql'}
					{#await import('$lib/components/GraphqlSchemaViewer.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							code={formatGraphqlSchema(contextElement.schema.schema)}
							class="h-full"
						/>
					{/await}
				{:else if contextElement.schema}
					<ObjectViewer json={formatSchema(contextElement.schema)} pureViewer collapseLevel={1} />
				{:else}
					<div class="text-tertiary">Not loaded yet</div>
				{/if}
			</div>
		{:else if contextElement.type === 'code' || contextElement.type === 'code_piece' || contextElement.type === 'diff'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<HighlightCode
					language={contextElement.lang}
					code={contextElement.content}
					class="w-full p-2 "
				/>
			</div>
		{/if}
	</svelte:fragment>
</Popover>
