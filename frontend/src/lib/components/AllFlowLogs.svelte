<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import JobLogs from './JobLogs.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import type { DurationStatus } from './graph'
	import type { Writable } from 'svelte/store'

	interface Props {
		states: Writable<Record<string, DurationStatus>> | undefined
	}

	let { states }: Props = $props()
</script>

<div class="flex flex-col">
	{#if states != undefined}
		{#each Object.entries($states ?? {}) as [id, status] (id)}
			<div class="pb-12"
				><h1 class="mb-2">Step {id}</h1>
				{#each Object.entries(status.byJob ?? {}) as jobS}
					{@const job = jobS[0]}
					<div>
						<a
							class="text-xs"
							rel="noreferrer"
							target="_blank"
							href="{base}/run/{job}?workspace={$workspaceStore}"
						>
							{job}
						</a><JobLogs jobId={job} /></div
					>
				{/each}
			</div>
		{/each}
	{:else}
		<Loader2 size={14} class="animate-spin " />
	{/if}
</div>
