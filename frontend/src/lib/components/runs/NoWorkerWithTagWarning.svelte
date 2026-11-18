<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { onDestroy, untrack } from 'svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { devWorkspaceTagSuffix, loadDevWorkspaceTagSuffix } from '$lib/devWorkspaceTags'

	const operatingWorkspace = useOperatingWorkspace()
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

	loadDevWorkspaceTagSuffix()
	let devSuffix = $derived(
		$devWorkspaceTagSuffix && customTag?.endsWith($devWorkspaceTagSuffix)
			? $devWorkspaceTagSuffix
			: undefined
	)

	async function lookForTag(): Promise<void> {
		try {
			if (!customTag) return
			const existsWorkerWithTag = await WorkerService.existsWorkersWithTags({
				tags: customTag,
				workspace: $operatingWorkspace
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
		untrack(() => {
			clearTimeout(timeout)
			timeout = setTimeout(() => lookForTag(), 2500)
		})
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
			{#if devSuffix}
				<br />It carries the <code>{devSuffix}</code> suffix of dev workspace and fork jobs: only worker
				groups flagged as serving dev workspaces listen to it. An admin can flag one on the Workers page.
			{/if}
		{/snippet}
	</Popover>
{/if}
