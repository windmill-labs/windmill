<script lang="ts">
	import { workspaceStore, userWorkspaces, userStore, type UserExt } from '$lib/stores'
	import { ScriptService } from '$lib/gen'
	import type { WorkspaceComparison } from '$lib/gen'
	import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
	import { Button } from './common'
	import { AlertTriangle, GitFork, CircleCheck, CircleX, Loader2 } from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import { onMount, untrack } from 'svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { childWorkspaceNoun, devLabelWord } from '$lib/utils/devWorkspaceLabel'
	import { diffActionableInDirection } from '$lib/utils_workspace_deploy'

	let loading = $state(false)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	/** Workspace `comparison` describes; control flow only, never rendered. */
	let comparisonFor: string | undefined = undefined
	let requestSeq = 0
	let error: string | undefined = $state(undefined)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	let parentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === parentWorkspaceId))
	// Detect fork/dev workspaces by their parent link, not the `wm-fork-` id prefix (dev
	// workspaces have an ordinary, prefix-less id). Keying on the parent (rather than the
	// prefix) also avoids a parentless "Fork of ()" banner when the linkage is dropped.
	let isFork = $derived(parentWorkspaceId != null)
	let isDevWorkspace = $derived(currentWorkspaceData?.is_dev_workspace ?? false)
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

	// Fork is fully in sync with its parent (comparison ran, no ahead/behind diffs).
	function isUpToDate(c: WorkspaceComparison | undefined): boolean {
		return !!c && !c.skipped_comparison && c.summary.total_diffs === 0
	}
	let upToDate = $derived(hasAnswer && isUpToDate(comparison))
	// Up to date with the parent but local drafts are pending — show the draft
	// state (same text + CTA as the draft banner) instead of "Everything is up to date".
	let showDraftsOnly = $derived(upToDate && draftCount > 0)

	// Leaving for a workspace with no comparison of its own has to invalidate whatever
	// is in flight too, or that response lands as this one's answer — and its CI counts
	// would be fetched for paths this workspace may not even have.
	function dropComparison() {
		requestSeq++
		comparison = undefined
		comparisonFor = undefined
		loading = false
		resetCiTestSummary()
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
			resetCiTestSummary()
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

	let ciTestPassing = $state(0)
	let ciTestFailing = $state(0)
	let ciTestRunning = $state(0)
	let ciTestTotal = $state(0)

	// These describe the rows of one comparison, so they are dropped with it.
	function resetCiTestSummary() {
		ciTestPassing = 0
		ciTestFailing = 0
		ciTestRunning = 0
		ciTestTotal = 0
	}

	async function fetchCiTestSummary() {
		if (!$workspaceStore || !comparison?.diffs) return
		const items = comparison.diffs
			.filter((d) => d.kind === 'script' || d.kind === 'flow' || d.kind === 'resource')
			.map((d) => ({ path: d.path, kind: d.kind as 'script' | 'flow' | 'resource' }))
		if (items.length === 0) return
		// Counted for the comparison current at call time — the poll below outlives a
		// fork switch, and a slow batch for the fork we left would repaint this one's.
		const seq = requestSeq
		try {
			const batch = await ScriptService.getCiTestResultsBatch({
				workspace: $workspaceStore,
				requestBody: { items }
			})
			if (seq !== requestSeq) return
			let passing = 0
			let failing = 0
			let running = 0
			let total = 0
			for (const results of Object.values(batch)) {
				for (const r of results) {
					total++
					if (r.status === 'success') passing++
					else if (r.status === 'failure' || r.status === 'canceled') failing++
					else if (r.status === 'running' || (r.job_id && !r.status)) running++
				}
			}
			ciTestPassing = passing
			ciTestFailing = failing
			ciTestRunning = running
			ciTestTotal = total
		} catch (e) {
			console.error('Failed to fetch CI test summary:', e)
		}
	}

	$effect(() => {
		if (comparison && comparison.summary.total_diffs > 0) {
			fetchCiTestSummary()
		}
	})

	// Poll while any CI test is still running
	$effect(() => {
		if (ciTestRunning <= 0) return
		const interval = setInterval(fetchCiTestSummary, 3000)
		return () => clearInterval(interval)
	})

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
	<!-- Side padding mirrors the page content container below, so the banner
	     stays aligned with it instead of bleeding to the viewport edges. -->
	<div class="w-full text-xs max-w-7xl mx-auto px-4 sm:px-8 pt-2">
		<div class="bg-blue-50 dark:bg-blue-900 rounded-md px-4 py-2">
			<!-- The summary wraps inside its own column while the CTA keeps its width and
			     stays on the first line: laid out as one non-wrapping row, the summary is
			     long enough on a laptop-width viewport to push the button out of the
			     banner instead of getting shorter. -->
			<div class="flex items-center justify-between gap-x-3">
				<div class="flex items-center flex-wrap gap-x-3 gap-y-1 min-w-0">
					<GitFork class="w-4 h-4 text-accent shrink-0" />
					<div class="text-sm min-w-0">
						<span class="font-medium text-blue-900 dark:text-blue-100">
							{isDevWorkspace
								? `${devLabelWord(currentWorkspaceData?.dev_workspace_label)} workspace of`
								: 'Fork of'}
							<b>{parentWorkspaceData?.name}</b
							>{#if parentWorkspaceData?.name !== parentWorkspaceId}
								({parentWorkspaceId}){/if}
						</span>
					</div>

					{#if loading}
						<span class="text-xs text-blue-600 dark:text-blue-400"> Checking for changes... </span>
					{:else if error}
						<span class="text-xs text-red-600 dark:text-red-400">
							{error}
						</span>
					{:else if comparison}
						<div class="flex items-center flex-wrap gap-x-4 gap-y-1 text-xs min-w-0">
							{#if comparison.summary.total_diffs > 0}
								<span class="text-blue-700 dark:text-blue-100">
									{forkAheadBehindMessage(changesAhead, changesBehind)}
									<span class="font-semibold underline">{parentWorkspaceId}</span> over {comparison
										.summary.total_diffs} items<span class="hidden lg:inline">:</span>
								</span>
								<!-- The per-kind breakdown is the first thing to go on a narrow
								     viewport: the item total above already sizes the change set, and
								     the compare page carries the detail. -->
								<div
									class="hidden lg:flex items-center flex-wrap gap-x-2 gap-y-1 whitespace-nowrap"
								>
									{#if comparison.summary.scripts_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.scripts_changed} script{comparison.summary
												.scripts_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.flows_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.flows_changed} flow{comparison.summary.flows_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.apps_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.apps_changed} app{comparison.summary.apps_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.resources_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.resources_changed} resource{comparison.summary
												.resources_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.variables_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.variables_changed} variable{comparison.summary
												.variables_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.resource_types_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.resource_types_changed} resource type{comparison.summary
												.resource_types_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.folders_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.folders_changed} folder{comparison.summary
												.folders_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.schedules_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.schedules_changed} schedule{comparison.summary
												.schedules_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.triggers_changed > 0}
										<span class="text-blue-700 dark:text-blue-100">
											{comparison.summary.triggers_changed} trigger{comparison.summary
												.triggers_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
								</div>

								{#if ciTestTotal > 0}
									-
									{#if ciTestFailing > 0}
										<div
											class="flex items-center gap-1 text-red-600 dark:text-red-400 whitespace-nowrap"
										>
											<CircleX class="w-3 h-3" />
											<span>CI: {ciTestFailing} failing</span>
										</div>
									{:else if ciTestRunning > 0}
										<div
											class="flex items-center gap-1 text-yellow-600 dark:text-yellow-400 whitespace-nowrap"
										>
											<Loader2 class="w-3 h-3 animate-spin" />
											<span>CI: {ciTestRunning} running</span>
										</div>
									{:else}
										<div
											class="flex items-center gap-1 text-green-600 dark:text-green-400 whitespace-nowrap"
										>
											<CircleCheck class="w-3 h-3" />
											<span>CI: {ciTestPassing} passing</span>
										</div>
									{/if}
								{/if}

								{#if comparison.summary.conflicts > 0}
									-
									<div
										class="flex items-center gap-1 text-orange-600 dark:text-orange-400 whitespace-nowrap"
									>
										<AlertTriangle class="w-3 h-3" />
										<span
											>{comparison.summary.conflicts} conflict{comparison.summary.conflicts !== 1
												? 's'
												: ''}</span
										>
									</div>
								{/if}
							{:else if comparison.skipped_comparison}
								<span class="text-blue-600 dark:text-blue-200">
									This {currentNoun} was created before the addition of certain windmill features, and
									therefore the changes with its parent workspace cannot be displayed.</span
								>
							{:else if showDraftsOnly}
								<span class="text-blue-700 dark:text-blue-100">
									This workspace has {draftCount} draft{draftCount !== 1 ? 's' : ''}
								</span>
							{:else}
								<span class="text-blue-600 dark:text-blue-200"> Everything is up to date </span>
							{/if}
						</div>
					{/if}
				</div>

				<div class="flex items-center gap-2 shrink-0">
					<Button
						variant="default"
						unifiedSize="sm"
						onclick={showDraftsOnly ? openDraftCompare : openComparisonDrawer}
					>
						{#if showDraftsOnly}
							Review & deploy drafts
						{:else if !hasAnswer || changesAhead > 0}
							Review & Deploy Changes
						{:else}
							Review & Update {currentNoun}
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</div>
{/if}
