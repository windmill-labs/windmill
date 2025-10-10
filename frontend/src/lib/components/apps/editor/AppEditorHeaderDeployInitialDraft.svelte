<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Path from '$lib/components/Path.svelte'

	let { summary, appPath, pathError = $bindable(), newEditedPath = $bindable() } = $props()

	let path: Path | undefined = $state(undefined)
	let dirtyPath = $state(false)
</script>

<Alert bgClass="mb-4" title="Require path" type="info">
	Choose a path to save the initial draft of the app.
</Alert>
<h3>Summary</h3>
<div class="w-full pt-2">
	<!-- svelte-ignore a11y_autofocus -->
	<input
		autofocus
		type="text"
		placeholder="App summary"
		class="text-sm w-full font-semibold"
		onkeydown={(e) => {
			e.stopPropagation()
		}}
		bind:value={$summary}
		onkeyup={() => {
			if ($appPath == '' && $summary?.length > 0 && !dirtyPath) {
				path?.setName(
					$summary
						.toLowerCase()
						.replace(/[^a-z0-9_]/g, '_')
						.replace(/-+/g, '_')
						.replace(/^-|-$/g, '')
				)
			}
		}}
	/>
</div>
<div class="py-2"></div>
<Path
	autofocus={false}
	bind:this={path}
	bind:error={pathError}
	bind:path={newEditedPath}
	bind:dirty={dirtyPath}
	initialPath=""
	namePlaceholder="app"
	kind="app"
/>
<div class="py-4"></div>
