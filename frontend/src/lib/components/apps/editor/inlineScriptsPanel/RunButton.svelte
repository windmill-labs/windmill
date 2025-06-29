<script lang="ts">
	import { getContext } from 'svelte'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		InlineScript
	} from '../../types'
	import { Button } from '$lib/components/common'
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'

	interface Props {
		id: string
		inlineScript?: InlineScript | undefined
		runLoading?: boolean
		hideShortcut?: boolean
	}

	let {
		id,
		inlineScript = undefined,
		runLoading = $bindable(false),
		hideShortcut = false
	}: Props = $props()

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { runnableJobEditorPanel } = getContext<AppEditorContext>('AppEditorContext')
	let cancelable: CancelablePromise<void>[] | undefined = $state(undefined)
</script>

{#if $runnableComponents[id] != undefined}
	{#if !runLoading}
		<Button
			loading={runLoading}
			size="xs"
			color="dark"
			btnClasses="!px-2 !py-1"
			on:click={async () => {
				runLoading = true
				$runnableJobEditorPanel.focused = true
				try {
					cancelable = $runnableComponents[id]?.cb?.map((f) => f(inlineScript, true))
					await Promise.all(cancelable)
				} catch {}
				runLoading = false
			}}
			shortCut={{ Icon: CornerDownLeft, hide: hideShortcut }}
		>
			Run
		</Button>
	{:else}
		<Button
			size="xs"
			color="red"
			variant="border"
			btnClasses="!px-2 !py-1 !ml-[3px]"
			on:click={async () => {
				cancelable?.forEach((f) => f.cancel())
				runLoading = false
			}}
		>
			<Loader2 size={12} class="animate-spin mr-2" />
			Cancel
		</Button>
	{/if}
{/if}
