<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { WorkspaceService, ScriptService } from '$lib/gen'
	import type { WorkspaceComparison } from '$lib/gen'
	import { Button } from './common'
	import { AlertTriangle, GitFork, CircleCheck, CircleX, Loader2 } from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import { onMount, untrack } from 'svelte'

	let loading = $state(false)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	let error: string | undefined = $state(undefined)

	let isFork = $derived($workspaceStore?.startsWith('wm-fork-') ?? false)
	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	let parentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === parentWorkspaceId))

	$effect(() => {
		;[$workspaceStore, parentWorkspaceId]
		untrack(() => {
			if (isFork && $workspaceStore) {
				checkForChanges()
			} else {
				comparison = undefined
			}
		})
	})

	onMount(() => {
		if (isFork && $workspaceStore) {
			checkForChanges()
		} else {
			comparison = undefined
		}
	})

	async function checkForChanges() {
		if (!$workspaceStore || !parentWorkspaceId) {
			return
		}

		loading = true
		error = undefined

		try {
			// Compare with parent workspace
			const result = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: $workspaceStore
			})

			comparison = result
		} catch (e) {
			console.error('Failed to compare workspaces:', e)
			error = `Failed to check for changes: ${e}`
			// Still show banner if there's an error, but with error message
		} finally {
			loading = false
		}
	}

	function openComparisonDrawer() {
		if (parentWorkspaceId && $workspaceStore) {
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore), {
				replaceState: true
			})
		}
	}

	let ciTestPassing = $state(0)
	let ciTestFailing = $state(0)
	let ciTestRunning = $state(0)
	let ciTestTotal = $state(0)

	async function fetchCiTestSummary() {
		if (!$workspaceStore || !comparison?.diffs) return
		const items = comparison.diffs
			.filter((d) => d.kind === 'script' || d.kind === 'flow' || d.kind === 'resource')
			.map((d) => ({ path: d.path, kind: d.kind as 'script' | 'flow' | 'resource' }))
		if (items.length === 0) return
		try {
			const batch = await ScriptService.getCiTestResultsBatch({
				workspace: $workspaceStore,
				requestBody: { items }
			})
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

	function forkAheadBehindMessage(changesAhead: number, changesBehind: number) {
		let msg: string[] = []
		if (changesAhead > 0 || changesBehind > 0) {
			msg.push('This fork is ')
			if (changesAhead > 0)
				msg.push(`${changesAhead} change${changesAhead > 1 ? 's' : ''} ahead of `)
			if (changesAhead > 0 && changesBehind > 0) msg.push('and ')
			if (changesBehind > 0)
				msg.push(`${changesBehind} change${changesBehind > 1 ? 's' : ''} behind `)
		}
		return msg.join('')
	}
</script>

{#if isFork}
	<div class="w-full bg-blue-50 dark:bg-blue-900 text-xs rounded-b-md max-w-7xl mx-auto">
		<div class="px-4 py-2">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<GitFork class="w-4 h-4 text-accent" />
					<div class="text-sm">
						<span class="font-medium text-blue-900 dark:text-blue-100">
							Fork of <b>{parentWorkspaceData?.name}</b> ({parentWorkspaceId})
						</span>
					</div>

					{#if loading}
						<span class="text-xs text-blue-600 dark:text-blue-400"> Checking for changes... </span>
					{:else if error}
						<span class="text-xs text-red-600 dark:text-red-400">
							{error}
						</span>
					{:else if comparison}
						<div class="flex items-center gap-4 text-xs">
							{#if comparison.summary.total_diffs > 0}
								<span class="text-blue-700 dark:text-blue-100">
									{forkAheadBehindMessage(
										comparison.summary.total_ahead,
										comparison.summary.total_behind
									)}
									<span class="font-semibold underline">{parentWorkspaceId}</span> over {comparison
										.summary.total_diffs} items:
								</span>
								<div class="flex items-center gap-2">
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
								</div>

								{#if ciTestTotal > 0}
									-
									{#if ciTestFailing > 0}
										<div class="flex items-center gap-1 text-red-600 dark:text-red-400">
											<CircleX class="w-3 h-3" />
											<span>CI: {ciTestFailing} failing</span>
										</div>
									{:else if ciTestRunning > 0}
										<div class="flex items-center gap-1 text-yellow-600 dark:text-yellow-400">
											<Loader2 class="w-3 h-3 animate-spin" />
											<span>CI: {ciTestRunning} running</span>
										</div>
									{:else}
										<div class="flex items-center gap-1 text-green-600 dark:text-green-400">
											<CircleCheck class="w-3 h-3" />
											<span>CI: {ciTestPassing} passing</span>
										</div>
									{/if}
								{/if}

								{#if comparison.summary.conflicts > 0}
									-
									<div class="flex items-center gap-1 text-orange-600 dark:text-orange-400">
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
									This fork was created before the addition of certain windmill features, and
									therefore the changes with its parent workspace cannot be displayed.</span
								>
							{:else}
								<span class="text-blue-600 dark:text-blue-200"> Everything is up to date </span>
							{/if}
						</div>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					<Button size="xs" color="blue" on:click={openComparisonDrawer}>
						{#if (comparison?.summary.total_ahead ?? 0) > 0}
							Review & Deploy Changes
						{:else}
							Review & Update fork
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</div>
{/if}
