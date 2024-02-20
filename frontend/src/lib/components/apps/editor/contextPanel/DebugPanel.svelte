<script lang="ts">
	import { getContext } from 'svelte'
	import { allItems } from '../../utils'
	import type { AppViewerContext } from '../../types'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	const { app, initialized } = getContext<AppViewerContext>('AppViewerContext')

	$: unintitializedComponents = allItems($app.grid, $app.subgrids)
		.map((x) => x.id)
		.filter((x) => !$initialized.initializedComponents?.includes(x))
		.sort()
		.join(', ')
</script>

<div class="flex items-center gap-2">
	{#if unintitializedComponents.length > 0}
		{#if unintitializedComponents.length > 0}
			<div class="flex gap-2 flex-col">
				<span class="text-xs font-semibold">Uninitialized components:</span>

				{#each unintitializedComponents as component}
					<div
						class="bg-indigo-500/90 border-indigo-600 text-xs border py-0.5 w-min px-2 rounded-sm text-white"
						>{component}</div
					>
				{/each}

				<Alert title="Uninitialized components" type="error" size="xs">
					The component is probably missing the code to initialize it. Please check the component's
					code.
				</Alert>
			</div>
		{/if}
	{/if}
</div>
