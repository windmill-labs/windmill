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
		/** May return a promise; the picker shows `value` again once it settles. */
		onchange: (value: string) => unknown
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

	const known = $derived(roleItems(roles, { admin }))
	// A role the workspace no longer defines (removed, or custom without a valid license) grants
	// nothing; it stays visible rather than showing nothing selected.
	const missing = $derived(value && !known.some((i) => i.value === value) ? value : undefined)
	const undefinedValue = $derived(roles ? missing : undefined)
	const items = $derived(
		missing
			? [...known, { label: undefinedValue ? `${missing} (inactive)` : missing, value: missing }]
			: known
	)

	let picks = $state(0)

	async function pick(v: string) {
		if (v === value) return
		try {
			await onchange(v)
		} finally {
			picks++
		}
	}

	const TOOLTIPS: Record<string, string> = {
		admin:
			'Full control over the workspace: members, settings, every item. Holds everything whatever the role.'
	}
</script>

{#if items.length <= 4 && !undefinedValue}
	<!-- The group keeps its own selection, so it is remounted after each pick: a pick the parent
	     refuses then shows `value` again instead of the refused one. -->
	{#key picks}
		<ToggleButtonGroup selected={value} {disabled} onSelected={pick}>
			{#snippet children({ item })}
				{#each items as i (i.value)}
					<ToggleButton value={i.value} label={i.label} tooltip={TOOLTIPS[i.value]} small {item} />
				{/each}
			{/snippet}
		</ToggleButtonGroup>
	{/key}
{:else}
	<Select
		{items}
		{disabled}
		{disablePortal}
		size="sm"
		class="min-w-36"
		bind:value={() => value, (v) => v && pick(v)}
	/>
{/if}
