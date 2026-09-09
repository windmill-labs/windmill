<!--
@component
The Tools section of the assistant settings modal: every tool definition this chat
sends with each turn. The list carries the model-facing name and description; opening
one shows the description in full and the arguments it takes. Read-only — tools are not
individually switchable, they follow the mode and the connected servers.
-->
<script lang="ts">
	import { Button, ListRow, Section } from '$lib/components/common'
	import { useListHighlight } from '$lib/components/common/listRow/listHighlight.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ArrowLeft } from 'lucide-svelte'
	import type { ToolSummary } from './agentContext'

	let {
		tools,
		active,
		blocksClose = $bindable()
	}: {
		tools: ToolSummary[]
		/** Whether this is the panel on screen. Gates the detail page's build, which pulls
		 * in the schema table and its syntax highlighter. */
		active: boolean
		/** True while the detail page is open, so the modal leaves Escape to this section. */
		blocksClose: boolean
	} = $props()

	type Marked = ToolSummary & { marked?: string }

	let filter = $state('')
	// The same fuzzy search the home list and the pickers use, run twice: `marked` covers
	// whatever the haystack was, and the name and the description are two fields on the
	// row rather than one string, so each needs a pass of its own to be highlighted.
	let nameHits = $state<Marked[] | undefined>(undefined)
	let descriptionHits = $state<Marked[] | undefined>(undefined)
	// The tool the detail page shows, kept when the page is left so the Right arrow
	// steps back into it.
	let selected = $state<ToolSummary | undefined>(undefined)
	let detailOpen = $state(false)

	const SEARCH_INPUT_ID = 'assistant-tools-search'
	const rowDomId = (index: number) => `assistant-tool-row-${index}`

	let searching = $derived(filter.trim().length > 0)
	// Name matches first, then the tools that only matched on what they do. Within each
	// half the order is the one uFuzzy ranked.
	let rows: { tool: ToolSummary; name?: string; description?: string }[] = $derived.by(() => {
		if (!searching) return tools.map((tool) => ({ tool }))
		const byName = nameHits ?? []
		const named = new Set(byName.map((t) => t.name))
		return [
			...byName.map((t) => ({ tool: t, name: t.marked })),
			...(descriptionHits ?? [])
				.filter((t) => !named.has(t.name))
				.map((t) => ({ tool: t, description: t.marked }))
		]
	})

	// The arrows and Enter are answered while focus stays in the search field, the way the
	// resource-type picker does it: type a few letters, step down the hits, open one.
	const highlight = useListHighlight({
		count: () => rows.length,
		rowId: rowDomId,
		// The top hit while searching — uFuzzy already ranked it there — and nothing lit
		// once the field is cleared, when every tool is on screen and none is the answer.
		restingIndex: () => (searching && rows.length > 0 ? 0 : -1),
		onActivate: (index) => open(rows[index]?.tool),
		activateEnterFrom: [SEARCH_INPUT_ID]
	})

	function open(tool: ToolSummary | undefined) {
		if (!tool) return
		selected = tool
		detailOpen = true
	}

	$effect(() => {
		blocksClose = detailOpen
	})
	// Parked with the section: a detail page left open behind another section would go
	// on reporting `blocksClose`, and the modal would refuse to close with nothing on
	// screen explaining why.
	$effect(() => {
		if (!active) detailOpen = false
	})

	/** Escape steps back to the list rather than closing the whole modal: `blocksClose`
	 * stops the modal's own handler, so this is the only thing left to answer the key. */
	function onKeydown(event: KeyboardEvent) {
		// Every section stays mounted while the modal is open, and `stopPropagation`
		// does nothing between listeners on `window`: without this, a key aimed at the
		// section on screen is answered by the four behind it too.
		if (!active || event.key !== 'Escape' || !detailOpen) return
		event.preventDefault()
		event.stopPropagation()
		detailOpen = false
	}

	/** Left and Right step between the two pages, which is `PagedContent` answering the
	 * arrows once it is given this. Forward only goes somewhere once a tool has been
	 * opened: the detail page has nothing to show before that. */
	function navigate(key: string) {
		if (key === 'list') detailOpen = false
		else if (selected) detailOpen = true
	}
</script>

<svelte:window onkeydown={onKeydown} />

<SearchItems {filter} items={tools} bind:filteredItems={nameHits} f={(t) => t.name} />
<SearchItems {filter} items={tools} bind:filteredItems={descriptionHits} f={(t) => t.description} />

<!-- The list and one tool are levels of one panel, the same shape the Skills and MCP
     panels use. Warmed once this panel is on screen so the schema table and its
     highlighter are built before the click rather than inside the transition. -->
<PagedContent
	warm={active}
	class="grow min-h-0"
	current={detailOpen ? 'detail' : 'list'}
	onNavigate={active ? navigate : undefined}
	pages={[
		{ key: 'list', content: listPage },
		{ key: 'detail', content: detailPage }
	]}
/>

{#snippet listPage()}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- Arrow keys and Enter are caught here so they work whether the search field or a row
	     holds focus. -->
	<div
		class="grow min-h-0 overflow-y-auto pr-2"
		onkeydown={highlight.onKeydown}
		onpointermove={highlight.pointerMoved}
	>
		<Section
			label="Tools"
			description="What the assistant can call in this session: the built-in tools, plus whatever the connected MCP servers expose."
		>
			<!-- Sticks to the top of the scrolling panel so a 70-row list stays searchable. -->
			<div class="sticky top-0 z-10 bg-surface pb-2">
				<TextInput
					bind:value={filter}
					size="sm"
					inputProps={{ placeholder: 'Search tools', id: SEARCH_INPUT_ID }}
				/>
			</div>
			{#if rows.length === 0}
				<div class="py-2 text-xs text-hint">
					{tools.length === 0 ? 'This chat carries no tools.' : 'No tool matches this search.'}
				</div>
			{:else}
				<!-- Borderless rows on their own hover, the shape the resource-type picker uses:
				     a card and dividers around 70 rows read as heavier than the list is. -->
				<div class="flex flex-col gap-0.5">
					{#each rows as row, index (row.tool.name)}
						{#snippet title()}
							<span class="truncate font-mono leading-5">
								{#if row.name}{@html row.name}{:else}{row.tool.name}{/if}
							</span>
						{/snippet}
						{#snippet subtitle()}
							{#if row.description}{@html row.description}{:else}{row.tool.description}{/if}
						{/snippet}
						<ListRow
							id={rowDomId(index)}
							{title}
							subtitle={row.tool.description ? subtitle : undefined}
							highlighted={index === highlight.index}
							onMouseEnter={() => highlight.hovered(index)}
							onClick={() => open(row.tool)}
						/>
					{/each}
				</div>
			{/if}
		</Section>
	</div>
{/snippet}

{#snippet detailPage()}
	<div class="grow min-h-0 overflow-y-auto pr-2">
		<!-- Sticky so the way back is always one click away, however far down the arguments go. -->
		<div class="flex sticky top-0 z-10 bg-surface pb-1">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: ArrowLeft }}
				btnClasses="text-secondary"
				onClick={() => (detailOpen = false)}
			>
				Tools
			</Button>
		</div>
		<!-- `headerClass` keeps a long tool name on one line. -->
		<Section
			label={selected?.name ?? ''}
			wrapperClass="mt-1"
			headerClass="min-w-0 truncate pr-2 font-mono"
			class="flex flex-col gap-4"
		>
			{#if selected?.description}
				<!-- In full, unlike the row, which truncates to one line: most of these run to
				     several sentences, and this page is where the rest of one lives. -->
				<div class="text-xs text-secondary whitespace-pre-line">{selected.description}</div>
			{/if}
			<SchemaViewer schema={selected?.parameters} />
		</Section>
	</div>
{/snippet}
