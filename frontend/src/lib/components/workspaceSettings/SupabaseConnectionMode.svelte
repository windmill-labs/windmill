<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import type { SupabaseConnectionMode } from './supabaseProvisioning'

	type Props = {
		mode: SupabaseConnectionMode
		onChange?: () => void
	}

	let { mode = $bindable(), onChange }: Props = $props()

	let open = $state(false)

	function set(v: SupabaseConnectionMode) {
		if (v === mode) return
		mode = v
		onChange?.()
	}

	const OPTIONS: { value: SupabaseConnectionMode; title: string; detail: string }[] = [
		{
			value: 'session',
			title: 'Session pooler',
			detail: 'Reaches Supabase over IPv4. Works from any worker.'
		},
		{
			value: 'direct',
			title: 'Direct connection',
			detail:
				'IPv6 only, unless the project has the IPv4 add-on. Workers on IPv4-only networks cannot reach it.'
		}
	]
</script>

<div class="border-t border-border-light pt-2">
	<button
		class="flex items-center gap-1 text-2xs text-secondary hover:text-primary"
		onclick={() => (open = !open)}
	>
		<ChevronRight size={12} class="transition-transform {open ? 'rotate-90' : ''}" />
		Connection mode: {mode === 'session' ? 'session pooler' : 'direct'}
	</button>
	{#if open}
		<div class="flex flex-col gap-1.5 mt-2">
			<!-- Not `RadioCard`: no radio dot, and selection reads through the accent surface. -->
			{#each OPTIONS as option (option.value)}
				{@const selected = mode === option.value}
				<button
					class="text-left border rounded-md p-2 transition-colors {selected
						? 'border-border-selected/50 bg-surface-accent-selected'
						: 'border-border-light hover:bg-surface-hover'}"
					onclick={() => set(option.value)}
				>
					<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}">
						{option.title}{option.value === 'session' ? ' · recommended' : ''}
					</span>
					<span class="block text-2xs text-secondary">{option.detail}</span>
				</button>
			{/each}
		</div>
	{/if}
</div>
