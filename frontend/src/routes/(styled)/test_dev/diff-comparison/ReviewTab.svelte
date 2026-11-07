<script lang="ts">
	import {
		diffAcceptRejectHunk,
		type DiffLineAnnotation,
		type FileDiffMetadata,
		type SelectedLineRange
	} from '@pierre/diffs'
	import type { Editor } from '@pierre/diffs/edit'
	import { Badge, Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import PierreDiff from '$lib/components/pierre/PierreDiff.svelte'
	import { randomUUID } from '$lib/utils/uuid'
	import { Check, Trash2, Undo2, Redo2, X } from 'lucide-svelte'
	import { caseFileDiff, type ComparisonCase } from './cases'
	import { lint } from './lint'
	import { REVIEW_AFTER, REVIEW_BEFORE } from './reviewSample'

	type Note =
		| { kind: 'change'; hunk: number; change: number; additions: number; deletions: number }
		| { kind: 'comment'; id: string }

	const reviewCase: ComparisonCase = {
		id: 'review',
		label: 'Review',
		filename: 'billing.ts',
		diff: { lang: 'typescript', before: REVIEW_BEFORE, after: REVIEW_AFTER }
	}

	// Raw: Pierre compares diffs by reference and mutates them in place.
	let reviewDiff: FileDiffMetadata = $state.raw(caseFileDiff(reviewCase))
	let comments: DiffLineAnnotation<Note>[] = $state.raw([])
	let commentText: Record<string, string> = $state({})
	let editing = $state(false)
	let editor: Editor<'file-diff', Note, undefined> | undefined = $state()
	let liveAfter: string | undefined = $state()
	let selected: SelectedLineRange | null = $state(null)
	let hoveredToken: string | undefined = $state()
	let log: string[] = $state([])

	function changeAnnotations(diff: FileDiffMetadata): DiffLineAnnotation<Note>[] {
		const out: DiffLineAnnotation<Note>[] = []
		diff.hunks.forEach((hunk, h) => {
			hunk.hunkContent.forEach((c, i) => {
				if (c.type !== 'change') return
				// Anchored under the block's last line; a pure deletion has no new-side line.
				const onAdditions = c.additions > 0
				out.push({
					side: onAdditions ? 'additions' : 'deletions',
					lineNumber: onAdditions
						? c.additionLineIndex + c.additions
						: c.deletionLineIndex + c.deletions,
					metadata: {
						kind: 'change',
						hunk: h,
						change: i,
						additions: c.additions,
						deletions: c.deletions
					}
				})
			})
		})
		return out
	}

	const annotations = $derived([...changeAnnotations(reviewDiff), ...comments])
	const resultText = $derived(reviewDiff.additionLines.join(''))
	const remaining = $derived(changeAnnotations(reviewDiff).length)
	const markers = $derived(lint(liveAfter ?? resultText))

	function resolve(hunk: number, change: number, type: 'accept' | 'reject') {
		reviewDiff = diffAcceptRejectHunk(reviewDiff, hunk, { type, changeIndex: change })
		log = [`${type} hunk ${hunk + 1}, change ${change + 1}`, ...log]
	}

	function resolveAll(type: 'accept' | 'reject') {
		// Resolving converts a change to context, so walk back to keep earlier indices valid.
		for (const a of changeAnnotations(reviewDiff).reverse()) {
			if (a.metadata.kind === 'change') {
				reviewDiff = diffAcceptRejectHunk(reviewDiff, a.metadata.hunk, {
					type,
					changeIndex: a.metadata.change
				})
			}
		}
		log = [`${type} all`, ...log]
	}

	function addComment(range: SelectedLineRange) {
		const id = randomUUID()
		commentText[id] = ''
		comments = [
			...comments,
			{
				side: range.endSide ?? range.side ?? 'additions',
				lineNumber: range.end,
				metadata: { kind: 'comment', id }
			}
		]
	}

	function reset() {
		reviewDiff = caseFileDiff(reviewCase)
		comments = []
		liveAfter = undefined
		log = []
	}

	const options = {
		diffStyle: 'unified' as const,
		enableLineSelection: true,
		enableGutterUtility: true,
		lineHoverHighlight: 'both' as const,
		onLineSelected: (range: SelectedLineRange | null) => (selected = range),
		onGutterUtilityClick: addComment,
		onTokenEnter: ({ tokenText }: { tokenText: string }) => (hoveredToken = tokenText),
		onTokenLeave: () => (hoveredToken = undefined)
	}
</script>

{#snippet headerMetadata(_diff: FileDiffMetadata)}
	<Badge color={remaining > 0 ? 'yellow' : 'green'}>
		{remaining > 0 ? `${remaining} to review` : 'reviewed'}
	</Badge>
{/snippet}

{#snippet note(a: DiffLineAnnotation<Note>)}
	{#if a.metadata.kind === 'change'}
		{@const m = a.metadata}
		<div class="flex items-center gap-2 border-y bg-surface-secondary px-3 py-1 text-xs">
			<span class="text-secondary">
				Change {m.change + 1} of hunk {m.hunk + 1}: +{m.additions} −{m.deletions}
			</span>
			<Button
				unifiedSize="2xs"
				variant="accent-secondary"
				startIcon={{ icon: Check }}
				disabled={editing}
				onclick={() => resolve(m.hunk, m.change, 'accept')}>Accept</Button
			>
			<Button
				unifiedSize="2xs"
				variant="subtle"
				startIcon={{ icon: X }}
				disabled={editing}
				onclick={() => resolve(m.hunk, m.change, 'reject')}>Reject</Button
			>
		</div>
	{:else}
		{@const id = a.metadata.id}
		<div class="flex items-center gap-2 border-y bg-surface-secondary px-3 py-2">
			<TextInput
				size="sm"
				bind:value={commentText[id]}
				inputProps={{ placeholder: `Comment on line ${a.lineNumber}` }}
			/>
			<Button
				unifiedSize="xs"
				variant="subtle"
				iconOnly
				startIcon={{ icon: Trash2 }}
				onclick={() => (comments = comments.filter((c) => c !== a))}
			/>
		</div>
	{/if}
{/snippet}

<div class="grid gap-4 xl:grid-cols-[3fr_2fr]">
	<section class="flex min-w-0 flex-col gap-2">
		<h2 class="text-sm font-semibold">Pierre: review an AI change</h2>
		<p class="text-2xs text-hint">
			Accept or reject each change block, add a comment from the gutter (+), select lines, hover a
			token, or edit the new side directly. Demo lint markers follow the edited text.
		</p>
		<div class="flex flex-wrap items-center gap-2">
			<Button
				unifiedSize="sm"
				variant="accent"
				disabled={editing || remaining === 0}
				onclick={() => resolveAll('accept')}>Accept all ({remaining})</Button
			>
			<Button
				unifiedSize="sm"
				variant="default"
				disabled={editing || remaining === 0}
				onclick={() => resolveAll('reject')}>Reject all</Button
			>
			<Button unifiedSize="sm" variant="subtle" onclick={reset}>Reset</Button>
			<Toggle size="xs" bind:checked={editing} options={{ right: 'Edit new side' }} />
			{#if editor}
				<Button
					unifiedSize="xs"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Undo2 }}
					onclick={() => editor?.undo()}
				/>
				<Button
					unifiedSize="xs"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Redo2 }}
					onclick={() => editor?.redo()}
				/>
			{/if}
		</div>
		<div class="flex gap-4 text-2xs text-secondary">
			<span>
				Selection: {selected ? `${selected.side ?? ''} ${selected.start}–${selected.end}` : 'none'}
			</span>
			<span>Hovered token: <code>{hoveredToken ?? '–'}</code></span>
			<span>Markers: {markers.length}</span>
		</div>
		<div class="h-[560px] overflow-auto border">
			<PierreDiff
				fileDiff={reviewDiff}
				lang="typescript"
				filename={reviewCase.filename}
				{options}
				{annotations}
				annotation={note}
				{headerMetadata}
				editable={editing}
				markers={editing ? markers : undefined}
				onEditor={(e) => (editor = e)}
				onEditChange={(text) => (liveAfter = text)}
				onEdited={(_text, fileDiff) => {
					reviewDiff = fileDiff
					liveAfter = undefined
					log = ['edit session accepted', ...log]
				}}
			/>
		</div>
	</section>

	<section class="flex min-w-0 flex-col gap-2">
		<h2 class="text-sm font-semibold">Monaco: editable diff</h2>
		<p class="text-2xs text-hint">
			Monaco's equivalent: the new side is editable and the gutter arrow reverts a change. No
			per-change accept, no inline comments.
		</p>
		<div class="h-[300px] overflow-hidden border">
			<DiffEditor
				className="h-full"
				defaultLang="typescript"
				defaultModifiedLang="typescript"
				defaultOriginal={REVIEW_BEFORE}
				defaultModified={REVIEW_AFTER}
				inlineDiff
				disableAutoInline
				open
			/>
		</div>
		<h3 class="text-xs font-semibold">Resulting file (Pierre side)</h3>
		<pre class="h-[160px] overflow-auto border p-2 text-2xs">{liveAfter ?? resultText}</pre>
		<h3 class="text-xs font-semibold">Events</h3>
		<ul class="h-[60px] overflow-auto text-2xs text-secondary">
			{#each log as entry, i (log.length - i)}<li>{entry}</li>{/each}
		</ul>
	</section>
</div>
