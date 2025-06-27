<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { onDestroy } from 'svelte'
	interface Props {
		tag: string
	}

	let { tag }: Props = $props()

	let noWorkerWithTag = $state(false)

	let timeout: NodeJS.Timeout | undefined = undefined

	let visible = true
	async function lookForTag(): Promise<void> {
		try {
			const existsWorkerWithTag = await WorkerService.existsWorkerWithTag({ tag })
			noWorkerWithTag = !existsWorkerWithTag
			if (noWorkerWithTag) {
				timeout = setTimeout(() => {
					if (visible) {
						lookForTag()
					}
				}, 1000)
			}
		} catch (err) {
			console.error(err)
		}
	}

	lookForTag()

	onDestroy(() => {
		visible = false
		if (timeout) {
			clearTimeout(timeout)
		}
	})
</script>

{#if noWorkerWithTag}
	<Popover notClickable placement="top">
		<AlertTriangle size={16} class="text-yellow-500" />
		{#snippet text()}
			No worker with tag <b>{tag}</b> is currently running.
		{/snippet}
	</Popover>
{/if}
