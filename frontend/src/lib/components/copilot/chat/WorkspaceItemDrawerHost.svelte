<script lang="ts">
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { aiChatManager } from './AIChatManager.svelte'

	/**
	 * Hosts the workspace-item drawers next to the chat. When a chat pill asks to open
	 * one, the host calls the matching editor's open method; when the pill is clicked
	 * again (toggle) or the chat manager wants to close, the host calls `closeDrawer`.
	 *
	 * Both editors are mounted unconditionally on purpose. Their internal `<Drawer>`
	 * wires close handling at mount time, and the resources page (which has worked for
	 * a long time) uses the same always-mounted pattern. Conditional mounting via
	 * `{#if}` was racy — the `on:close` handler captured an undefined reference and the
	 * X button stopped closing the drawer. The drawer markup stays hidden until
	 * `openDrawer()` is called, and the heavy editor body is still loaded lazily via
	 * `import()` inside `ResourceEditorDrawer`, so this isn't expensive.
	 */

	let variableEditor: VariableEditor | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()

	let lastVersion = -1

	$effect(() => {
		const t = aiChatManager.workspaceItemDrawer
		if (!t) return
		// React on every version bump (covers open, re-open, and toggle-close).
		if (t.version === lastVersion) return
		lastVersion = t.version
		if (!t.open) {
			variableEditor?.closeDrawer()
			resourceEditor?.closeDrawer()
			return
		}
		if (t.kind === 'variable' && variableEditor) {
			variableEditor.editVariable(t.path)
		} else if (t.kind === 'resource' && resourceEditor) {
			resourceEditor.initEdit(t.path)
		}
	})
</script>

<VariableEditor
	bind:this={variableEditor}
	on:close={() => aiChatManager.markWorkspaceItemDrawerClosed()}
/>
<ResourceEditorDrawer
	bind:this={resourceEditor}
	on:close={() => aiChatManager.markWorkspaceItemDrawerClosed()}
/>
