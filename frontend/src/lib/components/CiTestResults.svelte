<script lang="ts">
	import { ScriptService, type CiTestResult } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { CircleCheck, CircleX, Loader2, FlaskConical } from 'lucide-svelte'
	import { resource } from 'runed'

	interface Props {
		path: string
		kind: 'script' | 'flow' | 'resource'
	}

	let { path, kind }: Props = $props()

	let ciTests = resource(
		() => ({ workspace: $workspaceStore!, path, kind }),
		async (args) => {
			if (!args.workspace || !args.path) return []
			return ScriptService.getCiTestResults({
				workspace: args.workspace,
				path: args.path,
				kind: args.kind
			})
		}
	)

	let results: CiTestResult[] = $derived(ciTests.current ?? [])
	let hasResults = $derived(results.length > 0)
	let hasRunning = $derived(results.some((r) => r.status === 'running' || (r.job_id && !r.status)))

	// Poll while any test is still running
	$effect(() => {
		if (!hasRunning) return
		const interval = setInterval(() => ciTests.refetch(), 3000)
		return () => clearInterval(interval)
	})
</script>

{#if hasResults}
	<div class="flex flex-col gap-1 mb-4">
		<div class="flex items-center gap-1.5 text-xs font-semibold text-secondary">
			<FlaskConical size={14} />
			CI tests
		</div>
		<div class="flex flex-col gap-1">
			{#each results as test (test.test_script_path)}
				<div class="flex items-center gap-2 text-xs">
					{#if test.status === 'success'}
						<CircleCheck size={14} class="text-green-600" />
					{:else if test.status === 'failure' || test.status === 'canceled'}
						<CircleX size={14} class="text-red-600" />
					{:else if test.status === 'running'}
						<Loader2 size={14} class="text-yellow-600 animate-spin" />
					{:else}
						<div class="w-3.5 h-3.5 rounded-full border border-tertiary"></div>
					{/if}
					<a
						href="{base}/scripts/get/{test.test_script_path}?workspace={$workspaceStore}"
						class="text-secondary hover:text-primary truncate"
					>
						{test.test_script_path}
					</a>
					{#if test.job_id}
						<a
							href="{base}/run/{test.job_id}?workspace={$workspaceStore}"
							class="text-tertiary hover:text-secondary text-2xs"
						>
							view run
						</a>
					{/if}
				</div>
			{/each}
		</div>
	</div>
{/if}
