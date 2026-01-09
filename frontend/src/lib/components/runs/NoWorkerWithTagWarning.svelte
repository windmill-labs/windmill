<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { onDestroy, untrack } from 'svelte'
	interface Props {
		tag: string | undefined
		tagLabel?: string
	}

	let { tag, tagLabel = undefined }: Props = $props()

	let noWorkerWithTag = $state(false)

	let timeout: number | undefined = undefined

	let visible = true

	let customTag = $derived.by(() => {
		if (tag?.includes('$workspace') || tag?.includes('$args')) return

		if (tag?.includes('(')) {
			return tag.split('(')[0]
		}
		return tag
	})

	async function lookForTag(): Promise<void> {
		try {
			if (!customTag) return
			const existsWorkerWithTag = await WorkerService.existsWorkersWithTags({
				tags: customTag,
				workspace: $workspaceStore
			})
			noWorkerWithTag = !existsWorkerWithTag[customTag]
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

	$effect(() => {
		customTag
		untrack(() => timeout && setTimeout(() => lookForTag(), 2500))
	})

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
			No worker with {tagLabel ?? 'tag'} <b>{customTag}</b> is currently running.
		{/snippet}
	</Popover>
{/if}
