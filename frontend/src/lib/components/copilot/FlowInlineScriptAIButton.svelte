<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import type { ComponentProps } from 'svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import {
		getOpenInSessionHandoff,
		withEditorFlush
	} from '$lib/components/sessions/openInSessionContext'

	interface Props {
		moduleId?: string
		/** Materializes Monaco's in-flight keystrokes into the draft: this button sits in
		 * the code editor's own toolbar, so "type, then click" is the normal case. */
		flushEditor?: () => void
		btnProps?: ComponentProps<typeof Button>
	}

	const { moduleId, flushEditor, btnProps }: Props = $props()

	// The enclosing editor's "Open in AI session" hand-off, opening the preview on
	// the step this toolbar edits.
	const handoff = getOpenInSessionHandoff()
	const sessionSource = $derived.by(() => {
		const source = handoff?.source({ moduleId })
		return source && flushEditor ? withEditorFlush(source, flushEditor) : source
	})
</script>

<OpenInSessionButton source={sessionSource} {btnProps} />
