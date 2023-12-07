<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'

	export let isLoading = false
	export let tag: string | undefined = undefined

	let noWorkerWithTag = false

	$: if (tag && isLoading) {
		lookForTag(tag)
	} else {
		noWorkerWithTag = false
	}

	export async function lookForTag(tag: string): Promise<void> {
		try {
			const workers = (await WorkerService.listWorkers({ perPage: 1000 })).filter(
				(w) => w.last_ping !== undefined && w.last_ping < 60
			)

			noWorkerWithTag = workers.findIndex((w) => w.custom_tags?.includes(tag)) === -1
		} catch (err) {
			console.error(err)
		}
	}
</script>

{#if isLoading && tag && noWorkerWithTag}
	<Popover notClickable>
		<AlertTriangle size={16} class="text-yellow-500" />
		<svelte:fragment slot="text">
			No worker with tag <b>{tag}</b> is currently running.
		</svelte:fragment>
	</Popover>
{/if}
