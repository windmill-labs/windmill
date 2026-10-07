<script lang="ts">
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import ForkStatusStrip from './ForkStatusStrip.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import { workspaceStore, userWorkspaces, userStore, type UserExt } from '$lib/stores'
	import type { WorkspaceComparison } from '$lib/gen'
	import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
	import { goto } from '$app/navigation'
	import { onMount, untrack } from 'svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { childWorkspaceNoun } from '$lib/utils/devWorkspaceLabel'
	import { diffActionableInDirection } from '$lib/utils_workspace_deploy'

	let loading = $state(false)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	/** Workspace `comparison` describes; control flow only, never rendered. */
	let comparisonFor: string | undefined = undefined
	let requestSeq = 0
	let error: string | undefined = $state(undefined)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	// Detect fork/dev workspaces by their parent link, not the `wm-fork-` id prefix (dev
	// workspaces have an ordinary, prefix-less id). Keying on the parent (rather than the
	// prefix) also avoids a parentless "Fork of ()" banner when the linkage is dropped.
	let isFork = $derived(parentWorkspaceId != null)
	let currentNoun = $derived(childWorkspaceNoun(currentWorkspaceData))
	// Operators run scripts and flows, they never deploy a fork, so the banner and
	// its CTA are noise for them. Gates the fetches too, not just the markup: the
	// fork/parent comparison is an expensive tally no operator can act on.
	//
	// Only a role fetched for the workspace we are on answers this. `$workspaceStore`
	// flips synchronously on a switch while `$userStore` still holds the workspace we
	// left, so trusting it unqualified would flash the banner at (and start the tally
	// for) an operator entering a fork from a workspace where they are not one.
	function isConfirmedNonOperator(user: UserExt | undefined, ws: string | undefined): boolean {
		return !!user && !!ws && user.workspace_id === ws && !user.operator
	}
	let isNotOperator = $derived(isConfirmedNonOperator($userStore, $workspaceStore))
	let showBanner = $derived(isFork && isNotOperator)

	// Drafts in this fork. When the fork is otherwise in sync with its parent, a
	// user with only pending drafts should still get the draft CTA (mirrors the
	// non-fork WorkspaceDraftsBanner). Pass undefined when the banner is hidden so
	// it doesn't fetch.
	const drafts = useWorkspaceDrafts(() => (showBanner ? ($workspaceStore ?? undefined) : undefined))
	const draftCount = $derived(drafts.count)

	// Every read of `comparison` that decides what the banner says or where its button
	// goes must go through this: anything else — in flight, failed, or a tally skipped
	// outright — counts zero of everything, which is indistinguishable from "nothing to
	// deploy". Typed helper avoids the `never`-inference quirk on `$state` in `$derived`.
	function isAnswerable(c: WorkspaceComparison | undefined, isLoading: boolean): boolean {
		return !isLoading && !!c && !c.skipped_comparison
	}
	const hasAnswer = $derived(isAnswerable(comparison, loading))

	/** Bar width from which every count in the strip says what it counts. Below it they keep their
	 *  icon and their number, which is the part a reader is actually scanning for. Higher than the
	 *  width at which home puts its side trips in the bar, so the two do not arrive together. */
	const BAR_FITS_COUNT_LABELS = 1280
	const compactStrip = $derived(pageHeader.barWidth < BAR_FITS_COUNT_LABELS)

	// Fork is fully in sync with its parent (comparison ran, no ahead/behind diffs).
	function isUpToDate(c: WorkspaceComparison | undefined): boolean {
		return !!c && !c.skipped_comparison && c.summary.total_diffs === 0
	}
	let upToDate = $derived(hasAnswer && isUpToDate(comparison))
	// Up to date with the parent but local drafts are pending — show the draft
	// state (same text + CTA as the draft banner) instead of "Everything is up to date".
	let showDraftsOnly = $derived(upToDate && draftCount > 0)

	// Leaving for a workspace with no comparison of its own has to invalidate whatever
	// is in flight too, or that response lands as this one's answer.
	function dropComparison() {
		requestSeq++
		comparison = undefined
		comparisonFor = undefined
		loading = false
	}

	// `isNotOperator` is a dependency of its own: it only turns true once this
	// workspace's role has landed, which is after the switch that triggered it.
	$effect(() => {
		;[$workspaceStore, parentWorkspaceId, isNotOperator]
		untrack(() => {
			if (showBanner && $workspaceStore) {
				checkForChanges()
			} else {
				dropComparison()
			}
		})
	})

	onMount(() => {
		if (showBanner && $workspaceStore) {
			checkForChanges()
		} else {
			dropComparison()
		}
	})

	async function checkForChanges() {
		const ws = $workspaceStore
		const parent = parentWorkspaceId
		if (!ws || !parent) {
			return
		}

		// A comparison only ever describes the workspace it was requested for. The
		// component survives a fork switch, so drop the previous fork's rows before
		// fetching rather than let them answer for this one, and let only the newest
		// request write — responses can land out of order, and a late one would
		// otherwise paint another fork's counts over the current answer.
		if (comparisonFor !== ws) {
			comparison = undefined
			comparisonFor = undefined
		}
		const seq = ++requestSeq
		loading = true
		error = undefined

		try {
			// Compare with parent workspace (shared single-flight fetch — the chat
			// diff tool reuses this result instead of recomputing the comparison)
			const result = await fetchWorkspaceComparison(parent, ws)

			if (seq !== requestSeq) return
			comparison = result
			comparisonFor = ws
		} catch (e) {
			if (seq !== requestSeq) return
			console.error('Failed to compare workspaces:', e)
			error = `Failed to check for changes: ${e}`
			// Show the banner with the error rather than the rows we failed to refresh:
			// on a switch those belong to the fork we just left.
			comparison = undefined
			comparisonFor = undefined
		} finally {
			if (seq === requestSeq) loading = false
		}
	}

	// Opens the direction the button offers, so the label and the list agree: a fork
	// with nothing to deploy lands on the update side, not on an empty deploy list.
	/** The compare page opened on one direction, for a badge that counts exactly that. The button
	 *  beside them still chooses on the reader's behalf; a badge means the one thing it names. */
	function openCompareDir(dir: 'deploy_to' | 'update') {
		if (!parentWorkspaceId || !$workspaceStore) return
		goto(
			'/forks/compare?workspace_id=' +
				encodeURIComponent($workspaceStore) +
				(dir === 'update' ? '&dir=update' : ''),
			{ replaceState: true }
		)
	}

	function openComparisonDrawer() {
		if (parentWorkspaceId && $workspaceStore) {
			const dir = hasAnswer && changesAhead === 0 ? '&dir=update' : ''
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + dir, {
				replaceState: true
			})
		}
	}

	function openDraftCompare() {
		if ($workspaceStore) {
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + '&mode=draft', {
				replaceState: true
			})
		}
	}

	// Counted with the compare page's own predicate so the banner never advertises a
	// direction whose list is empty: the `ahead`/`behind` sums in the summary include
	// rows a direction does not carry, and miss a parent-only row that the update
	// direction carries at `behind = 0`.
	function countDir(c: WorkspaceComparison | undefined, mergeIntoParent: boolean): number {
		return c?.diffs.filter((d) => diffActionableInDirection(d, mergeIntoParent)).length ?? 0
	}
	const changesAhead = $derived(countDir(comparison, true))
	const changesBehind = $derived(countDir(comparison, false))

	function forkAheadBehindMessage(changesAhead: number, changesBehind: number) {
		let msg: string[] = []
		if (changesAhead > 0 || changesBehind > 0) {
			msg.push(`This ${currentNoun} is `)
			if (changesAhead > 0)
				msg.push(`${changesAhead} change${changesAhead > 1 ? 's' : ''} ahead of `)
			if (changesAhead > 0 && changesBehind > 0) msg.push('and ')
			if (changesBehind > 0)
				msg.push(`${changesBehind} change${changesBehind > 1 ? 's' : ''} behind `)
		}
		return msg.join('')
	}
</script>

{#if showBanner}
	<!-- In the band, not a banner: the breadcrumb already says which fork this is, so all this has
	     to carry is how far it has drifted and the way to review that. The per-kind breakdown, the
	     CI line and the conflict count live on the compare page the button opens. -->
	<PageHeaderContent actions={forkAction} />
{/if}

{#snippet forkAction()}
	{@const total = comparison?.summary.total_diffs ?? 0}
	{@const conflicts = comparison?.summary.conflicts ?? 0}
	<!-- What the comparison has to say stands beside the strip, never instead of it: a fork still
	     being counted, or one whose count failed, is exactly when a reader wants the page that does
	     the counting properly. -->
	{#if loading}
		<span class="text-2xs text-tertiary">Checking for changes…</span>
	{:else if error}
		<span class="text-2xs text-red-600 dark:text-red-400" title={error}>Comparison failed</span>
	{/if}
	<!-- The counts and the way to review them are one control: every segment opens the same page on
	     the part it names, so they share a box rather than sitting beside each other as four
	     unrelated marks. -->
	<ForkStatusStrip
		ahead={changesAhead}
		behind={changesBehind}
		{conflicts}
		drafts={draftCount}
		{parentWorkspaceId}
		noun={currentNoun}
		compact={compactStrip}
		onOpen={(target) =>
			target === 'drafts' ? openDraftCompare() : openCompareDir(target)}
		action={showDraftsOnly
			? {
					label: 'Review & deploy drafts',
					title: `Review and deploy this workspace's drafts`,
					onclick: openDraftCompare
				}
			: {
					label: !hasAnswer || changesAhead > 0 ? 'Review changes' : `Review & Update ${currentNoun}`,
					title: hasAnswer
						? `${forkAheadBehindMessage(changesAhead, changesBehind)} ${parentWorkspaceId} over ${total} items`
						: `Review this ${currentNoun} against ${parentWorkspaceId}`,
					onclick: openComparisonDrawer
				}}
	/>
{/snippet}
