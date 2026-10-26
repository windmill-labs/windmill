<script lang="ts">
	import type { Snippet } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { slideDynamic } from '$lib/transitions'

	// One optional setting of a flow, script, app or step: a toggle, and the setting's
	// parameters under it while it is on. The settings surfaces build their rows from this so
	// they keep one spacing and indent.

	interface Props {
		label: string
		tooltip?: string
		documentationLink?: string
		/** Whether the setting is configured; its parameters show while it is. */
		checked: boolean | undefined
		/** The new state, for the host to write or clear the setting's value. */
		onchange?: (checked: boolean) => void
		disabled?: boolean
		/** Badges the toggle "EE" while it is disabled. */
		eeOnly?: boolean
		/** Lets a host scroll to the row: `[data-setting="<key>"]`. */
		setting?: string
		aiId?: string
		aiDescription?: string
		/** Shown under the row whatever its state, e.g. why it is disabled. */
		hint?: Snippet
		/** The setting's parameters. */
		children?: Snippet
	}

	let {
		label,
		tooltip,
		documentationLink,
		checked,
		onchange,
		disabled = false,
		eeOnly = false,
		setting,
		aiId,
		aiDescription,
		hint,
		children
	}: Props = $props()

	// Hosts read `checked` from the value, and a value being typed passes through states that read
	// as off (an emptied field, a 0, an empty expression). Once its parameters have focus, they
	// stay open until the toggle is clicked; the toggle itself shows whether the value is set.
	let editing = $state(false)
</script>

<div class="flex flex-col gap-2" data-setting={setting}>
	<Toggle
		size="xs"
		bind:checked={
			() => checked,
			(v) => {
				editing = false
				// The host writes the value here, before the row opens. A click runs microtasks between
				// the input's listeners, so a `change` listener would come after the parameters mounted
				// on the old value, and an editor that reads its code once at mount would open empty.
				onchange?.(!!v)
			}
		}
		{disabled}
		{eeOnly}
		{aiId}
		{aiDescription}
		options={{ right: label, rightTooltip: tooltip, rightDocumentationLink: documentationLink }}
	/>
	{#if hint}
		<!-- pl-9 lines everything under the label: an xs toggle is 28px wide plus its 8px gap. -->
		<div class="pl-9 text-2xs text-secondary">{@render hint()}</div>
	{/if}
	{#if (checked || editing) && children}
		<!-- The slide clamps this box's height. A flex column would shrink an `overflow` child (an
		     editor) to fit the clamp, so the slide would measure it short and jump at the end. -->
		<div transition:slideDynamic onfocusin={() => (editing = true)}>
			<div class="flex flex-col gap-2 pl-9">
				{@render children()}
			</div>
		</div>
	{/if}
</div>
