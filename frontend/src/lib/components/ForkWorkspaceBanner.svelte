<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import type { WorkspaceComparison } from '$lib/gen'
	import { Button } from './common'
	import { AlertTriangle, GitFork } from 'lucide-svelte'
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

	function forkAheadBehindMessage(
		changesAhead: number,
		changesBehind: number,
	) {
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
	<div class="w-full bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800">
		<div class="px-4 py-2">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<GitFork class="w-4 h-4 text-blue-600 dark:text-blue-400" />
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
								<span>
									{forkAheadBehindMessage(
										comparison.summary.total_ahead,
										comparison.summary.total_behind,
									)}
									<span class="font-semibold underline">{parentWorkspaceId}</span> over {comparison.summary
										.total_diffs} items:
								</span>
								<div class="flex items-center gap-2">
									{#if comparison.summary.scripts_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.scripts_changed} script{comparison.summary
												.scripts_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.flows_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.flows_changed} flow{comparison.summary.flows_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.apps_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.apps_changed} app{comparison.summary.apps_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.resources_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.resources_changed} resource{comparison.summary
												.resources_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
									{#if comparison.summary.variables_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.variables_changed} variable{comparison.summary
												.variables_changed !== 1
												? 's'
												: ''}
										</span>
									{/if}
								</div>

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
								<span class="text-blue-600 dark:text-blue-400"> This fork was created before the addition of certain windmill features, and therefore the changes with its parent workspace cannot be displayed.</span>
							{:else}
								<span class="text-blue-600 dark:text-blue-400"> Everything is up to date </span>
							{/if}
						</div>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					{#if comparison && comparison.summary.total_diffs > 0}
						<Button size="xs" color="blue" on:click={openComparisonDrawer}>
							{#if comparison.summary.total_ahead > 0}
								Review & Deploy Changes
							{:else}
								Review & Update fork
							{/if}
						</Button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
