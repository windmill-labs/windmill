<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import { tick, type ComponentProps } from 'svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { getOpenInSessionHandoff } from '$lib/components/sessions/openInSessionContext'

	interface Props {
		moduleId?: string
		/** Materializes Monaco's in-flight keystrokes into the draft. This button
		 * sits in the code editor's own toolbar, so "type, then click" is the
		 * normal case, and the session preview loads the item from its draft —
		 * without this the last (sub-second) edits would not be in it. */
		flushEditor?: () => void
		btnProps?: ComponentProps<typeof Button>
	}

	const { moduleId, flushEditor, btnProps }: Props = $props()

	// The enclosing editor's "Open in AI session" hand-off, opening the preview on
	// the step this toolbar edits.
	const handoff = getOpenInSessionHandoff()
	const sessionSource = $derived.by(() => {
		const source = handoff?.source({ moduleId })
		if (!source || !flushEditor) return source
		return {
			...source,
			beforeOpen: async () => {
				flushEditor()
				// The flush lands in the draft store through an effect; let it run
				// before the hand-off persists that store.
				await tick()
				await source.beforeOpen?.()
			}
		}
	})
</script>

<OpenInSessionButton source={sessionSource} {btnProps} />
