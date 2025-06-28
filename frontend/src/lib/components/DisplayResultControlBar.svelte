<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { Download, InfoIcon, ClipboardCopy, Expand } from 'lucide-svelte'
	import Popover from './Popover.svelte'
	import { copyToClipboard } from '$lib/utils'
	import type { DisplayResultUi } from './custom_ui'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		customUi?: DisplayResultUi | undefined
		filename?: string | undefined
		workspaceId?: string | undefined
		jobId?: string | undefined
		nodeId?: string | undefined
		base: string
		result: any
		disableTooltips?: boolean
	}

	let {
		customUi = undefined,
		filename = undefined,
		workspaceId = undefined,
		jobId = undefined,
		nodeId = undefined,
		base,
		result,
		disableTooltips = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	function toJsonStr(result: any) {
		try {
			return JSON.stringify(result ?? null, null, 4) ?? 'null'
		} catch (e) {
			return 'error stringifying object: ' + e.toString()
		}
	}
</script>

<div class={twMerge('flex flex-row gap-2.5 z-10 text-tertiary -mt-1 items-center')}>
	{#if customUi?.disableDownload !== true}
		<a
			download="{filename ?? 'result'}.json"
			class="text-current"
			href={workspaceId && jobId
				? nodeId
					? `${base}/api/w/${workspaceId}/jobs/result_by_id/${jobId}/${nodeId}`
					: `${base}/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
				: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`}
		>
			<Download size={14} />
		</a>
	{/if}
	{#if disableTooltips !== true}
		<Popover documentationLink="https://www.windmill.dev/docs/core_concepts/rich_display_rendering">
			{#snippet text()}
				The result renderer in Windmill supports rich display rendering, allowing you to customize
				the display format of your results.
			{/snippet}
			<div>
				<InfoIcon size={14} />
			</div>
		</Popover>
	{/if}
	<button onclick={() => copyToClipboard(toJsonStr(result))}>
		<ClipboardCopy size={14} />
	</button>
	<button onclick={() => dispatch('open-drawer')}>
		<Expand size={14} />
	</button>
</div>
