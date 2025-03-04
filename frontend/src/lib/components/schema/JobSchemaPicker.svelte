<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'

	export let job: any
	export let payloadData: any | undefined = undefined
	export let hovering = false
</script>

<SchemaPickerRow {payloadData} date={job.created_at} {hovering}>
	<svelte:fragment slot="start">
		<div class="center-center">
			<div
				class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}"
				title={job.success ? 'Success' : 'Failed'}
			/>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="extra">
		<div class="center-center {hovering ? '' : '!hidden'}">
			<a
				target="_blank"
				href="{base}/run/{job.id}?workspace={$workspaceStore}"
				class="text-right float-right text-secondary"
				title="See run detail in a new tab"
			>
				<ExternalLink size={16} />
			</a>
		</div>
	</svelte:fragment>
</SchemaPickerRow>
