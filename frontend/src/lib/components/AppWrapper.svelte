<script lang="ts">
	import AppEditor from './apps/editor/AppEditor.svelte'
	import type { AppEditorProps } from './apps/types'

	let { app: oldApp, ...props }: AppEditorProps = $props()

	let app = $state(oldApp)

	$effect(() => {
		app = oldApp
	})
</script>

<AppEditor {app} {...props} />
