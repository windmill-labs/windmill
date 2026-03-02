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
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import type { FlowModule } from '$lib/gen'

	interface Props {
		contextElement: ContextElement
		deletable?: boolean
		onDelete?: () => void
	}

	let { contextElement, deletable = false, onDelete }: Props = $props()
	const icon = ContextIconMap[contextElement.type]
	let showDelete = $state(false)

	const isDeletable = $derived(deletable && contextElement.deletable !== false)
</script>

<Popover>
	{#snippet trigger()}
		<div
			class={twMerge(
				'border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs cursor-default hover:bg-surface-hover hover:cursor-pointer max-w-48 bg-surface'
			)}
			onmouseenter={() => (showDelete = true)}
			onmouseleave={() => (showDelete = false)}
			aria-label="Context element"
			role="button"
			tabindex={0}
		>
			<button onclick={isDeletable ? onDelete : undefined} class:cursor-default={!isDeletable}>
				{#if showDelete && isDeletable}
					<X size={16} />
				{:else if contextElement.type === 'flow_module' || contextElement.type === 'flow_module_code_piece'}
					<FlowModuleIcon module={contextElement as FlowModule} size={16} />
				{:else}
					{@const SvelteComponent = icon}
					<SvelteComponent size={16} />
				{/if}
			</button>
			<span class="truncate">
				{contextElement.type === 'diff'
					? contextElement.title.replace(/_/g, ' ')
					: contextElement.title}
			</span>
		</div>
	{/snippet}
	{#snippet content()}
		{#if contextElement.type === 'error'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<Highlight language={json} code={contextElement.content} class="w-full p-2" />
			</div>
		{:else if contextElement.type === 'db'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				{#if contextElement.schema && contextElement.schema.lang === 'graphql'}
					{#await Promise.all([import('$lib/components/GraphqlSchemaViewer.svelte'), formatGraphqlSchema(contextElement.schema.schema)])}
						<Loader2 class="animate-spin" />
					{:then [Module, code]}
						<Module.default {code} class="h-full" />
					{/await}
				{:else if contextElement.schema}
					<ObjectViewer json={formatSchema(contextElement.schema)} pureViewer collapseLevel={1} />
				{:else}
					<div class="text-primary">Not loaded yet</div>
				{/if}
			</div>
		{:else if contextElement.type === 'code' || contextElement.type === 'code_piece' || contextElement.type === 'diff' || contextElement.type === 'flow_module_code_piece'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<HighlightCode
					language={contextElement.lang}
					code={contextElement.content}
					className="w-full p-2 "
				/>
			</div>
		{:else if contextElement.type === 'flow_module'}
			{#if contextElement.value.content}
				<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
					<HighlightCode
						language={contextElement.value.language}
						code={contextElement.value.content}
						className="w-full p-2 "
					/>
				</div>
			{:else}
				<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
					<div class="text-primary">{contextElement.title}</div>
				</div>
			{/if}
		{:else if contextElement.type === 'app_frontend_file'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<HighlightCode
					language={contextElement.path.endsWith('.tsx') || contextElement.path.endsWith('.ts')
						? 'bun'
						: contextElement.path.endsWith('.css')
							? 'bash'
							: 'bun'}
					code={contextElement.content}
					className="w-full p-2"
				/>
			</div>
		{:else if contextElement.type === 'app_backend_runnable'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				{#if contextElement.runnable.inlineScript}
					<HighlightCode
						language={contextElement.runnable.inlineScript.language}
						code={contextElement.runnable.inlineScript.content}
						className="w-full p-2"
					/>
				{:else}
					<ObjectViewer json={contextElement.runnable} pureViewer collapseLevel={2} />
				{/if}
			</div>
		{:else if contextElement.type === 'app_code_selection'}
			<div class="max-w-96 max-h-[300px] text-xs overflow-auto">
				<div class="text-tertiary text-xs mb-1 px-2 pt-1">
					{contextElement.source} (L{contextElement.startLine}-L{contextElement.endLine})
				</div>
				<HighlightCode
					language="bun"
					code={contextElement.content}
					className="w-full p-2"
				/>
			</div>
		{:else if contextElement.type === 'app_datatable'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				<div class="text-tertiary text-xs mb-1">
					{contextElement.datatableName}/{contextElement.schemaName === 'public'
						? ''
						: contextElement.schemaName + ':'}{contextElement.tableName}
				</div>
				<ObjectViewer json={contextElement.columns} pureViewer collapseLevel={1} />
			</div>
		{/if}
	{/snippet}
</Popover>
