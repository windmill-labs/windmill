<script lang="ts">
	import DispatchEventsTable from './DispatchEventsTable.svelte'
	import { useDispatchEvents } from './useDispatchEvents.svelte'

	type Props = { workspace: string; jobId: string }
	let { workspace, jobId }: Props = $props()

	const events = useDispatchEvents(
		() => workspace,
		() => jobId
	)

	const list = $derived(events.list)
</script>

{#if list.length > 0}
	<div class="mr-2 sm:mr-0 mt-12 mb-6">
		<h3 class="text-xs font-semibold text-emphasis mb-1">Dispatch</h3>
		<div class="border rounded-md overflow-hidden">
			<DispatchEventsTable events={list} {workspace} />
		</div>
	</div>
{/if}
