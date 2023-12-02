<script lang="ts">
	import { getContext } from 'svelte'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		InlineScript
	} from '../../types'
	import { Button, Kbd } from '$lib/components/common'
	import { getModifierKey } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'

	export let id: string
	export let inlineScript: InlineScript | undefined = undefined
	export let runLoading = false
	export let hideShortcut = false

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const { runnableJobEditorPanel } = getContext<AppEditorContext>('AppEditorContext')
	let cancelable: CancelablePromise<void>[] | undefined = undefined
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
		>
			<div class="flex flex-row gap-1 items-center">
				Run
				{#if !hideShortcut}
					<div class="flex flex-row items-center">
						<Kbd small isModifier>{getModifierKey()}</Kbd>
						<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
					</div>
				{/if}
			</div>
		</Button>
	{:else}
		<Button
			size="xs"
			color="red"
			variant="border"
			btnClasses="!px-2 !py-1.5"
			on:click={async () => {
				cancelable?.forEach((f) => f.cancel())
				runLoading = false
			}}
		>
			<Loader2 size={14} class="animate-spin mr-2" />
			Cancel
		</Button>
	{/if}
{/if}
