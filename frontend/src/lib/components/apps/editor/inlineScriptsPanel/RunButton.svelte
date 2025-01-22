<script lang="ts">
	import { getContext } from 'svelte'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		InlineScript
	} from '../../types'

	import RunButtonInner from './RunButtonInner.svelte'

	export let id: string
	export let inlineScript: InlineScript | undefined = undefined
	export let runLoading = false
	export let hideShortcut = false

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { runnableJobEditorPanel } = getContext<AppEditorContext>('AppEditorContext')
	let cancelable: CancelablePromise<void>[] | undefined = undefined

	const onRun = async () => {
		try {
			$runnableJobEditorPanel.focused = true
			cancelable = $runnableComponents[id]?.cb?.map((f) => f(inlineScript, true))
			await Promise.all(cancelable)
		} catch {}
	}

	const onCancel = async () => {
		cancelable?.forEach((f) => f.cancel())
	}
</script>

{#if runnableComponents && $runnableComponents[id] != undefined}
	<RunButtonInner bind:runLoading {hideShortcut} {onRun} {onCancel} />
{/if}
