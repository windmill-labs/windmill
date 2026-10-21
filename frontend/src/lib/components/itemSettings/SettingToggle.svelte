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
		checked = $bindable(),
		onchange,
		disabled = false,
		eeOnly = false,
		setting,
		aiId,
		aiDescription,
		hint,
		children
	}: Props = $props()
</script>

<div class="flex flex-col gap-2" data-setting={setting}>
	<Toggle
		size="xs"
		bind:checked
		{disabled}
		{eeOnly}
		{aiId}
		{aiDescription}
		on:change={(e) => onchange?.(e.detail)}
		options={{ right: label, rightTooltip: tooltip, rightDocumentationLink: documentationLink }}
	/>
	{#if hint}
		<!-- pl-9 lines everything under the label: an xs toggle is 28px wide plus its 8px gap. -->
		<div class="pl-9 text-2xs text-secondary">{@render hint()}</div>
	{/if}
	{#if checked && children}
		<div class="flex flex-col gap-2 pl-9" transition:slideDynamic>
			{@render children()}
		</div>
	{/if}
</div>
