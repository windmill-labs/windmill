<script lang="ts">
	import type { Writable } from 'svelte/store'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { ACCENT_COLOR_SETTING, parseAccentColor } from '$lib/accentColor'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const DEFAULT_PICK = '#1f9d55'

	let raw = $derived($values[ACCENT_COLOR_SETTING])
	// An empty value is deleted on save, so it reads as off rather than as an empty color.
	let enabled = $derived(typeof raw === 'string' && raw !== '')
</script>

<div class="flex flex-col gap-2">
	<Toggle
		{disabled}
		bind:checked={
			() => enabled, (v) => ($values[ACCENT_COLOR_SETTING] = v ? DEFAULT_PICK : undefined)
		}
		options={{ right: 'Use a custom accent color' }}
	/>
	{#if enabled}
		<div class="flex items-center gap-2">
			<input
				type="color"
				class="!w-16 h-8 shrink-0 cursor-pointer"
				aria-label="Accent color"
				{disabled}
				value={parseAccentColor(raw) ?? DEFAULT_PICK}
				oninput={(e) => ($values[ACCENT_COLOR_SETTING] = e.currentTarget.value)}
			/>
			<div class="w-32">
				<TextInput
					inputProps={{ disabled, placeholder: DEFAULT_PICK }}
					bind:value={() => raw ?? '', (v) => ($values[ACCENT_COLOR_SETTING] = String(v).trim() || undefined)}
				/>
			</div>
		</div>
		<span class="text-xs text-secondary">
			Accents keep the default palette's lightness and take this color's hue, so text stays readable
			whatever color you pick. Applies to every user after saving.
		</span>
	{/if}
</div>
