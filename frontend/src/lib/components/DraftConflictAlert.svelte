<script lang="ts">
	import { Alert, Button } from '$lib/components/common'

	interface Props {
		/** Take the draft the server holds, replacing what is on screen. */
		onReload: () => void
		/** Write what is on screen over the server's draft. */
		onOverwrite: () => void
		/** A resolution is in flight: neither answer is offered again until it settles. */
		busy?: boolean
	}

	let { onReload, onOverwrite, busy = false }: Props = $props()
</script>

<Alert type="warning" title="Your draft changed elsewhere">
	<div class="flex flex-col items-start gap-2">
		<div>
			It was saved from another tab or session since this one read it, so your changes here are no
			longer being saved.
		</div>
		<div class="flex flex-row gap-2">
			<Button unifiedSize="sm" variant="default" disabled={busy} onClick={onReload}>
				Load the other version
			</Button>
			<Button unifiedSize="sm" variant="default" disabled={busy} onClick={onOverwrite}>
				Keep mine
			</Button>
		</div>
	</div>
</Alert>
