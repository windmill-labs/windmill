<script lang="ts">
	import { getContext } from 'svelte'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		InlineScript
	} from '../../types'
	import { RunButton } from '$lib/components/common'

	export let id: string
	export let inlineScript: InlineScript | undefined = undefined
	export let runLoading = false
	export let hideShortcut = false
	export let size: 'xs' | 'xs2' = 'xs'
	export let btnClasses = ''

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { runnableJobEditorPanel } = getContext<AppEditorContext>('AppEditorContext')
	let cancelable: CancelablePromise<void>[] | undefined = undefined
</script>

{#if $runnableComponents[id] != undefined}
	<RunButton
		{btnClasses}
		testIsLoading={runLoading}
		{hideShortcut}
		on:cancel={async () => {
			cancelable?.forEach((f) => f.cancel())
			runLoading = false
		}}
		on:run={async () => {
			runLoading = true
			$runnableJobEditorPanel.focused = true
			try {
				cancelable = $runnableComponents[id]?.cb?.map((f) => f(inlineScript, true))
				await Promise.all(cancelable)
			} catch {}
			runLoading = false
		}}
		{size}
	/>
{/if}
