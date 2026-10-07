<!--
@component
How far a fork has drifted and the way to review it, as one control rather than a row of separate
marks: the counts and the button share a box, divided but not spaced, and only the two ends of that
box are rounded.

They are one thing — every segment opens the same page on the part it names — so they read as one
shape. Separate badges beside a separate button say four unrelated things happen to sit together.
-->
<script lang="ts">
	import { AlertTriangle, ArrowDown, ArrowUp, Pencil } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	/** What a segment opens: a direction of the comparison, or the drafts beside it. */
	export type ForkStatusTarget = 'deploy_to' | 'update' | 'drafts'

	interface Props {
		/** Items this fork has that its parent lacks. */
		ahead?: number
		/** Items its parent has that it lacks. */
		behind?: number
		/** Items that are both, which have to be resolved before either direction can carry them. */
		conflicts?: number
		/** Work in this fork that is not deployed anywhere yet. */
		drafts?: number
		/** Opens the page on the part a segment names. */
		onOpen: (target: ForkStatusTarget) => void
		/** The end cap: the segment that opens the whole comparison. */
		action: { label: string; title: string; onclick: () => void }
		/** Names the parent in the counts' tooltips. Nullable, like the workspace row it comes from. */
		parentWorkspaceId?: string | null
		/** What this workspace is called in prose — "fork", or a dev workspace's own word. */
		noun?: string
		/** The bar is short of room: every segment drops its word and keeps its icon and its count.
		 *  What each one means is in its tooltip either way. */
		compact?: boolean
	}

	let {
		ahead = 0,
		behind = 0,
		conflicts = 0,
		drafts = 0,
		onOpen,
		action,
		parentWorkspaceId = '',
		noun = 'fork',
		compact = false
	}: Props = $props()

	// The compare page's own tones, so a count in the bar and the rows behind it look alike. Drafts
	// share blue with behind and are told apart by the pencil and by naming what they count.
	// With room, a segment says what it counts; short of it, the icon and the number carry the
	// meaning and the word goes. The icons are the compare page's own, and the tooltip names the
	// count either way.
	const SEGMENT =
		'inline-flex items-center gap-1 px-2 text-2xs whitespace-nowrap transition-colors'
	// Each count carries its own fill, the compare page's colour for what it counts. Behind and
	// drafts share blue and are told apart by their icon.
	const TONES = {
		ahead:
			'bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-700/40 dark:text-green-100 dark:hover:bg-green-700/60',
		behind:
			'bg-blue-50 text-blue-800 hover:bg-blue-100 dark:bg-blue-700/40 dark:text-blue-100 dark:hover:bg-blue-700/60',
		conflict:
			'bg-orange-100 text-orange-800 hover:bg-orange-200 dark:bg-orange-700/40 dark:text-orange-100 dark:hover:bg-orange-700/60',
		draft:
			'bg-blue-50 text-blue-800 hover:bg-blue-100 dark:bg-blue-700/40 dark:text-blue-100 dark:hover:bg-blue-700/60'
	} as const
	const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`
	/** Whether any count stands before the end cap, which is what its rule divides it from. With
	 *  nothing to its left the cap is the whole strip and a leading rule would divide nothing. */
	const anyCount = $derived(ahead > 0 || behind > 0 || conflicts > 0 || drafts > 0)
</script>

<!-- `overflow-hidden` on the box is what rounds the two ends and nothing else: each segment keeps
     square corners and the box clips them, so no segment has to know where in the row it sits. One
     outline around the row and nothing between the segments: their own colours already tell them
     apart, and a rule between each pair would cut the row back into the chips this replaced. -->
<div
	class="inline-flex items-stretch h-6 rounded-md border bg-surface-tertiary overflow-hidden"
>
	{#if ahead > 0}
		<button
			type="button"
			class={twMerge(SEGMENT, TONES.ahead)}
			title="{ahead} ahead of {parentWorkspaceId} — review what this {noun} has to give"
			onclick={() => onOpen('deploy_to')}
		>
			<ArrowUp class="w-3 h-3" />
			{ahead}{compact ? '' : ' ahead'}
		</button>
	{/if}
	{#if behind > 0}
		<button
			type="button"
			class={twMerge(SEGMENT, TONES.behind)}
			title="{behind} behind {parentWorkspaceId} — review what this {noun} has yet to take"
			onclick={() => onOpen('update')}
		>
			<ArrowDown class="w-3 h-3" />
			{behind}{compact ? '' : ' behind'}
		</button>
	{/if}
	{#if conflicts > 0}
		<!-- A conflict is ahead and behind at once, so it has no direction of its own; it opens the
		     side it is resolved from before deploying. -->
		<button
			type="button"
			class={twMerge(SEGMENT, TONES.conflict)}
			title="{plural(conflicts, 'conflicting item')} — review them"
			onclick={() => onOpen('deploy_to')}
		>
			<AlertTriangle class="w-3 h-3" />
			{compact ? conflicts : plural(conflicts, 'conflict')}
		</button>
	{/if}
	{#if drafts > 0}
		<button
			type="button"
			class={twMerge(SEGMENT, TONES.draft)}
			title="{plural(
				drafts,
				'draft'
			)} in this {noun}, not deployed anywhere yet — review and deploy"
			onclick={() => onOpen('drafts')}
		>
			<Pencil class="w-3 h-3" />
			{compact ? drafts : plural(drafts, 'draft')}
		</button>
	{/if}
	<!-- The end cap carries the whole comparison, and it is here whatever the counts say: a fork
	     still being counted is exactly when a reader wants the page that counts properly. The strip's
	     one rule is the one before it: everything to its left is a reading, this is an action. It
	     keeps its words at every width — the counts have icons to fall back on and it has none. -->
	<button
		type="button"
		class={twMerge(
			SEGMENT,
			'bg-surface-tertiary text-secondary hover:bg-surface-hover hover:text-primary font-medium',
			anyCount && 'border-l'
		)}
		title={action.title}
		onclick={action.onclick}
	>
		{action.label}
	</button>
</div>
