<script lang="ts">
	import { untrack } from 'svelte'
	import AppEditor from './apps/editor/AppEditor.svelte'
	import type { AppEditorProps } from './apps/types'

	let { app: oldApp, ...props }: AppEditorProps = $props()

	let app = $state(untrack(() => oldApp))
</script>

<AppEditor {app} {...props} />
