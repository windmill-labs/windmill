<script lang="ts">
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import type { EffectiveRole } from '$lib/gen'
	import { roleItems } from '$lib/workspaceRoles'

	interface Props {
		/** The workspace's roles; undefined while loading. */
		roles: EffectiveRole[] | undefined
		/** A role name, or `admin`. */
		value: string | undefined
		onchange: (value: string) => void
		/** Whether Admin is offered next to the roles. */
		admin?: boolean
		disabled?: boolean
		disablePortal?: boolean
	}

	let {
		roles,
		value,
		onchange,
		admin = true,
		disabled = false,
		disablePortal = false
	}: Props = $props()

	const items = $derived.by(() => {
		const list = roleItems(roles, { admin })
		// Keep a value the list no longer has visible, rather than showing nothing selected.
		if (value && !list.some((i) => i.value === value)) list.push({ label: value, value })
		return list
	})

	const TOOLTIPS: Record<string, string> = {
		admin:
			'Full control over the workspace: members, settings, every item. Holds everything whatever the role.'
	}
</script>

{#if items.length <= 4}
	<ToggleButtonGroup selected={value} {disabled} onSelected={(v) => onchange(v)}>
		{#snippet children({ item })}
			{#each items as i (i.value)}
				<ToggleButton value={i.value} label={i.label} tooltip={TOOLTIPS[i.value]} small {item} />
			{/each}
		{/snippet}
	</ToggleButtonGroup>
{:else}
	<Select
		{items}
		{disabled}
		{disablePortal}
		size="sm"
		class="min-w-36"
		bind:value={() => value, (v) => v && v !== value && onchange(v)}
	/>
{/if}
