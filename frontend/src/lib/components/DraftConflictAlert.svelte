<script lang="ts">
	import { Alert } from '$lib/components/common'

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

<Alert
	type="warning"
	title="Your draft changed elsewhere"
	actions={[
		{ label: 'Load the other version', variant: 'default', disabled: busy, onClick: onReload },
		{ label: 'Keep mine', variant: 'default', disabled: busy, onClick: onOverwrite }
	]}
>
	It was saved from another tab or session since this one read it, so your changes here are no longer
	being saved.
</Alert>
