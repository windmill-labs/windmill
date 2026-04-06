<script lang="ts">
	import { ScriptService, type CiTestResult } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { CircleCheck, CircleX, Loader2, TestTubes } from 'lucide-svelte'
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
</script>

{#if hasResults}
	<div class="flex flex-col gap-1">
		<div class="flex items-center gap-1.5 text-xs font-semibold text-secondary">
			<TestTubes size={14} />
			CI Tests
		</div>
		<div class="flex flex-col gap-1">
			{#each results as test (test.test_script_path)}
				<div class="flex items-center gap-2 text-xs">
					{#if test.status === 'success'}
						<Badge color="green" small>
							<CircleCheck size={12} class="mr-1" />
							pass
						</Badge>
					{:else if test.status === 'failure' || test.status === 'canceled'}
						<Badge color="red" small>
							<CircleX size={12} class="mr-1" />
							fail
						</Badge>
					{:else if test.status === 'running'}
						<Badge color="yellow" small>
							<Loader2 size={12} class="mr-1 animate-spin" />
							running
						</Badge>
					{:else}
						<Badge small>pending</Badge>
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
