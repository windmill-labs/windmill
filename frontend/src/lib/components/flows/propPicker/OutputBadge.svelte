<script lang="ts">
	import type { Job } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'

	export let job: Job | undefined = undefined
</script>

{#if job}
	<div
		class={twMerge(
			'flex flex-row items-center gap-1 rounded-md bg-green-100 p-1',
			'success' in job && job.success ? 'bg-green-100' : 'bg-red-100'
		)}
	>
		<span class="text-xs truncate relative" dir="rtl">
			{job.id}
		</span>
		<a
			target="_blank"
			href="{base}/run/{job.id}?workspace={$workspaceStore}"
			class="text-right float-right text-secondary px-2"
			title="See run detail in a new tab"
		>
			<ExternalLink size={12} class="text-secondary" />
		</a>
	</div>
{/if}
