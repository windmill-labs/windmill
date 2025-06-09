<script lang="ts">
	import type { Job } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { base } from '$lib/base'
	import { ExternalLink } from 'lucide-svelte'

	export let job:
		| Job
		| { id: string; result: unknown; type: 'CompletedJob'; success: boolean; workspace_id: string }
		| undefined = undefined
	export let small: boolean = false
</script>

{#if job}
	<div
		class={twMerge(
			'flex flex-row w-fit items-center justify-between gap-2 rounded-md bg-surface-secondary group',
			small ? 'p-0.5 px-1.5' : 'p-1 px-2',
			$$props.class
		)}
	>
		<div
			class={twMerge(
				'rounded-full',
				small ? 'w-1.5 h-1.5' : 'w-2 h-2',
				'success' in job && job.success ? 'bg-green-400' : 'bg-red-400'
			)}
			title={'success' in job && job.success ? 'Success' : 'Failed'}
		></div>

		<span
			class={`${small ? 'text-[10px]' : 'text-xs'} truncate`}
			dir="rtl"
			title={`job id: ${job.id}`}
		>
			{job.id.slice(-5)}
		</span>
		<a
			target="_blank"
			href="{base}/run/{job.id}?workspace={job.workspace_id}"
			class="text-right float-right text-gray-300 group-hover:text-gray-400 transition-all duration-200 dark:text-gray-500 dark:group-hover:text-gray-300"
			title="See run detail in a new tab"
		>
			<ExternalLink size={small ? 10 : 12} />
		</a>
	</div>
{/if}
