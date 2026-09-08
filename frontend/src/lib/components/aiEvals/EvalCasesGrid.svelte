<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import EditableTextarea from '$lib/components/common/EditableTextarea.svelte'
	import { Button, EmptyState } from '$lib/components/common'
	import { ListPlus, Plus, Trash2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import type { CaseDraft } from './evalUtils'

	let {
		cases = $bindable(),
		onRemove,
		onAdd,
		focusCaseId = undefined,
		locked = false
	}: {
		/** The drawer's working copy. Edits land here as they are made; the drawer writes them. */
		cases: CaseDraft[]
		/** Asked before a row goes, since a stored case has runs that executed it. */
		onRemove: (c: CaseDraft) => void
		/** Adds a case. The empty table offers this itself, where the first row would be. */
		onAdd: () => void
		/** A case to open for typing as soon as it appears — the one just added, so a new row is
		 *  ready to be filled in rather than waiting to be clicked. */
		focusCaseId?: string
		/** The cases are being written: an edit made now is one the request already left behind. */
		locked?: boolean
	} = $props()

	/** A dataset is capped at 1000 cases and every row here mounts two editors and a button, so the
	 *  whole set at once is thousands of components for a table you read a screenful of. ag-grid
	 *  virtualised its rows; `DataTable` paginates instead, which is the primitive already here. */
	const CASES_PER_PAGE = 25
	let currentPage = $state(1)
	/** Bound to `DataTable`'s own per-page selector, so changing it there re-slices here rather
	 *  than only relabelling the footer. */
	let perPage = $state(CASES_PER_PAGE)
	let lastPage = $derived(Math.max(1, Math.ceil(cases.length / perPage)))
	let pageCases = $derived(cases.slice((currentPage - 1) * perPage, currentPage * perPage))
	// A case removed from the last page leaves it empty; step back rather than show nothing.
	$effect(() => {
		if (currentPage > lastPage) currentPage = lastPage
	})

	/** `expected` is whatever the case holds: a bare string, or a value shown as JSON. Read back
	 *  the same way, so a case that held an object keeps holding one. */
	function expectedToText(value: unknown): string {
		if (value == undefined) return ''
		return typeof value === 'string' ? value : JSON.stringify(value, null, 2)
	}

	/** The question editors, by case id, so a newly added row can be opened. */
	let questionEditors = $state<Record<string, EditableTextarea | undefined>>({})
	$effect(() => {
		const id = focusCaseId
		if (!id) return
		untrack(() => {
			// Onto the page the case is on before reaching for its editor: paginated, a case added to
			// a full page is not rendered yet, and there would be nothing to open.
			const at = cases.findIndex((c) => c.id === id)
			if (at < 0) return
			currentPage = Math.floor(at / perPage) + 1
			// After the row has been rendered and registered itself.
			requestAnimationFrame(() => questionEditors[id]?.edit())
		})
	})

	function setExpected(c: CaseDraft, text: string) {
		// Cleared means the case has no expected answer, which is not the same as expecting the
		// empty string: a scorer reads `undefined` as "nothing to measure here" and leaves the case
		// out of its mean, where `''` scores it a hard zero.
		const trimmed = text.trim()
		if (!trimmed) {
			c.expected = undefined
			return
		}
		try {
			c.expected = JSON.parse(text)
		} catch {
			c.expected = text
		}
	}
</script>

{#if cases.length === 0}
	<EmptyState
		icon={ListPlus}
		title="No cases yet"
		description="A case is one question this agent is asked, and the answer it is measured against. A dataset needs at least one before it can be saved or run."
		action={{
			label: 'Add a case',
			icon: Plus,
			onClick: onAdd,
			// The same lock the header's own button takes: a save reads the case list before it is
			// sent, so a case added while one is in flight would be dropped when it lands.
			disabled: locked,
			// The only live call to action here: Save stays disabled until a case exists.
			variant: 'accent'
		}}
	/>
{:else}
	<DataTable
		size="xs"
		tableFixed
		containerClass="bg-surface-tertiary"
		paginated={cases.length > perPage}
		bind:currentPage
		bind:perPage
		rowCount={cases.length}
		hasMore={currentPage < lastPage}
		showPrev={currentPage > 1}
		on:next={() => (currentPage = Math.min(currentPage + 1, lastPage))}
		on:previous={() => (currentPage = Math.max(currentPage - 1, 1))}
	>
		<colgroup>
			<col style="width: 55%" />
			<col />
			<col style="width: 3rem" />
		</colgroup>
		<Head>
			<tr>
				<Cell head first>Question</Cell>
				<Cell head>Expected</Cell>
				<Cell head last></Cell>
			</tr>
		</Head>
		<tbody class="divide-y">
			{#each pageCases as c (c.id)}
				<Row>
					<Cell first>
						<!-- `commitOnInput`: the drawer's Save reads this list, so an edit has to be in it by
					     the time the button is pressed rather than waiting on a blur that the press
					     itself would cause. -->
						<EditableTextarea
							bind:this={questionEditors[c.id ?? '']}
							value={c.input?.user_message ?? ''}
							placeholder="Question"
							editable={!locked}
							commitOnInput
							class="w-full"
							textClass="text-xs font-normal"
							onSave={(v) => (c.input = { ...(c.input ?? {}), user_message: v })}
						/>
					</Cell>
					<Cell>
						<EditableTextarea
							value={expectedToText(c.expected)}
							placeholder="Expected"
							editable={!locked}
							commitOnInput
							class="w-full"
							textClass="text-xs font-normal"
							onSave={(v) => setExpected(c, v)}
						/>
					</Cell>
					<Cell last numeric>
						<Button
							unifiedSize="sm"
							variant="subtle"
							destructive
							iconOnly
							startIcon={{ icon: Trash2 }}
							title="Delete case"
							disabled={locked}
							onclick={() => onRemove(c)}
						/>
					</Cell>
				</Row>
			{/each}
		</tbody>
	</DataTable>
{/if}
