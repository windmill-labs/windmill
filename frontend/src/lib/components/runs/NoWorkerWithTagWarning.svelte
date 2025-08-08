<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { onDestroy } from 'svelte'
	interface Props {
		tag: string
		tagLabel?: string
	}

	let { tag, tagLabel = undefined }: Props = $props()

	let noWorkerWithTag = $state(false)

	let timeout: NodeJS.Timeout | undefined = undefined

	let visible = true
	async function lookForTag(): Promise<void> {
		try {
			const existsWorkerWithTag = await WorkerService.existsWorkersWithTags({ tags: tag })
			noWorkerWithTag = !existsWorkerWithTag[tag]
			if (noWorkerWithTag) {
				timeout = setTimeout(() => {
					if (visible) {
						lookForTag()
					}
				}, 2500)
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
			No worker with {tagLabel ?? 'tag'} <b>{tag}</b> is currently running.
		{/snippet}
	</Popover>
{/if}
