<script lang="ts">
	import { untrack } from 'svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { Loader2, X } from 'lucide-svelte'
	import { ContextIconMap, formatAppDatatableContextTitle, type ContextElement } from './context'
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
	import { getFileIcon } from '$lib/components/icons/fileIcon'

	interface Props {
		contextElement: ContextElement
		deletable?: boolean
		onDelete?: () => void
		/** Render context-picker / inspector picks (see COMPACT_TYPES) in a smaller
		 * size — used above a sent message; the composer keeps every badge the
		 * same size. */
		compact?: boolean
	}

	let { contextElement, deletable = false, onDelete, compact = false }: Props = $props()

	// Context-picker / inspector picks that render smaller above a sent message
	// (the composer keeps every badge the same size).
	const COMPACT_TYPES = new Set([
		'app_dom_selector',
		'workspace_app',
		'workspace_script',
		'workspace_flow',
		'attached_file'
	])
	const icon = ContextIconMap[untrack(() => contextElement).type]
	let showDelete = $state(false)

	const isDeletable = $derived(deletable && contextElement.deletable !== false)
</script>

<Popover>
	{#snippet trigger()}
		{@const label =
			contextElement.type === 'diff'
				? contextElement.title.replace(/_/g, ' ')
				: contextElement.title}
		{@const small = compact && COMPACT_TYPES.has(contextElement.type)}
		{@const iconSize = small ? 12 : 16}
		<div
			class={twMerge(
				'border rounded-md flex flex-row items-center text-primary font-normal cursor-default hover:bg-surface-hover hover:cursor-pointer max-w-48 bg-surface',
				small ? 'px-1 py-0 gap-0.5 text-2xs' : 'px-1 py-0.5 gap-1 text-xs'
			)}
			onmouseenter={() => (showDelete = true)}
			onmouseleave={() => (showDelete = false)}
			aria-label="Context element"
			role="button"
			tabindex={0}
		>
			<button onclick={isDeletable ? onDelete : undefined} class:cursor-default={!isDeletable}>
				{#if showDelete && isDeletable}
					<X size={iconSize} />
				{:else if contextElement.type === 'flow_module' || contextElement.type === 'flow_module_code_piece'}
					<FlowModuleIcon module={contextElement as FlowModule} size={iconSize} />
				{:else if contextElement.type === 'attached_file'}
					{@const fileIcon = getFileIcon(contextElement.title)}
					{@const FileIconComponent = fileIcon.icon}
					<FileIconComponent size={iconSize} class={fileIcon.className ?? ''} />
				{:else}
					{@const SvelteComponent = icon}
					<SvelteComponent size={iconSize} />
				{/if}
			</button>
			<span class="truncate" title={label}>{label}</span>
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
				<HighlightCode language="bun" code={contextElement.content} className="w-full p-2" />
			</div>
		{:else if contextElement.type === 'app_datatable'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				<div class="text-tertiary text-xs mb-1">
					{formatAppDatatableContextTitle(
						contextElement.datatableName,
						contextElement.schemaName,
						contextElement.tableName
					)}
				</div>
				<ObjectViewer json={contextElement.columns} pureViewer collapseLevel={1} />
			</div>
		{:else if contextElement.type === 'app_dom_selector'}
			<div class="p-2 max-w-96 text-xs overflow-auto">
				<div class="text-tertiary mb-1">Selected preview element</div>
				<div class="font-mono break-all">{contextElement.selector}</div>
			</div>
		{:else if contextElement.type === 'attached_file'}
			<div class="p-2 max-w-96 max-h-[300px] text-xs overflow-auto">
				<pre class="whitespace-pre-wrap break-words font-mono text-2xs"
					>{contextElement.content.length > 5000
						? contextElement.content.slice(0, 5000) + '\n…'
						: contextElement.content}</pre
				>
			</div>
		{/if}
	{/snippet}
</Popover>
