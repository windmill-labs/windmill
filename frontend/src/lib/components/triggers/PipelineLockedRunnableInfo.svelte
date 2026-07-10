<script lang="ts">
	import { Alert } from '$lib/components/common'

	// Read-only viewer shown in place of the runnable ScriptPicker when a
	// trigger drawer is opened in the pipeline-editor context (the editor
	// was called with a fixed script path). Pipeline membership means the
	// script's path is part of the pipeline's structure — re-pointing the
	// trigger at a different script would silently detach this node from
	// the pipeline.
	interface Props {
		path: string
	}
	let { path }: Props = $props()
</script>

<div class="flex flex-col gap-2 mb-2">
	<div
		class="border rounded-md text-xs px-3 py-2 text-secondary bg-surface-input font-mono break-all"
	>
		{path}
	</div>
	<Alert type="info" title="Pipeline-bound trigger" size="xs">
		Triggers whose runnable is part of a data pipeline are managed from the pipeline editor.
		The script path is fixed; edit the pipeline to reassign or remove the trigger.
	</Alert>
</div>
