<script lang="ts">
	import { getContext } from 'svelte'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		InlineScript
	} from '../../types'

	import RunButtonInner from '$lib/components/RunButton.svelte'

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
	let cancelable: CancelablePromise<void>[] | undefined = undefined

	const onRun = async () => {
		runLoading = true
		try {
			$runnableJobEditorPanel.focused = true
			cancelable = $runnableComponents[id]?.cb?.map((f) => f(inlineScript, true))
			await Promise.all(cancelable)
		} catch {}
		runLoading = false
	}

	const onCancel = async () => {
		cancelable?.forEach((f) => f.cancel())
		runLoading = false
	}
</script>

{#if runnableComponents && $runnableComponents[id] != undefined}
	<RunButtonInner isLoading={runLoading} {hideShortcut} {onRun} {onCancel} />
{/if}
