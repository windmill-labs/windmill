<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import SelectDropdown from './select/SelectDropdown.svelte'
	import { clickOutside } from '$lib/utils'

	let {
		role,
		roles,
		onSelect
	}: {
		/** The role in effect, shown on the badge. */
		role: string
		/** The roles the caller may switch to. */
		roles: string[]
		onSelect: (role: string) => void
	} = $props()

	let open = $state(false)
	let btnEl: HTMLButtonElement | undefined = $state()
	const items = $derived(roles.map((r) => ({ label: r, value: r })))

	// The table picker's drawer opens at `disposables + 10000`, which the
	// dropdown's own z-index would sit under.
	const dropdownClass = 'z-[20000]'
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="relative flex min-w-0" use:clickOutside={{ onClickOutside: () => (open = false) }}>
	<button
		bind:this={btnEl}
		type="button"
		class="flex items-center gap-0.5 rounded-md pl-2 pr-1 py-0.5 text-2xs min-w-0
			bg-surface-sunken text-primary transition-[filter,transform]
			hover:brightness-95 active:brightness-90 active:scale-[0.97]
			{open ? 'brightness-95' : ''}"
		onclick={(e) => {
			// The row underneath folds on click, and picking a role is not that.
			e.stopPropagation()
			open = !open
		}}
	>
		<!-- A long role name gives way rather than pushing the row's own actions
		     past its right edge. -->
		<span class="truncate">{role}</span>
		<ChevronDown
			size={11}
			class="shrink-0 text-secondary transition-transform {open ? 'rotate-180' : ''}"
		/>
	</button>
	<SelectDropdown
		processedItems={items}
		value={role}
		{open}
		listAutoWidth={false}
		class={dropdownClass}
		getInputRect={btnEl && (() => btnEl!.getBoundingClientRect())}
		onSelectValue={(item) => {
			open = false
			if (item.value !== role) onSelect(item.value)
		}}
	/>
</span>
