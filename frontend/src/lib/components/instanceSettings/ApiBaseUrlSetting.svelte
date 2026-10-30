<script lang="ts">
	import TextInput from '../text_input/TextInput.svelte'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import { API_BASE_URL_SURFACES, type ApiBaseUrlSurface } from '$lib/apiBaseUrl.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		placeholder?: string
		disabled?: boolean
	}

	let { values, placeholder, disabled = false }: Props = $props()

	const allSurfaces = API_BASE_URL_SURFACES.map((s) => s.value)

	// Unset means every surface, including ones added later.
	let appliesTo = $derived<ApiBaseUrlSurface[]>(
		Array.isArray($values['api_base_url_applies_to'])
			? $values['api_base_url_applies_to']
			: allSurfaces
	)

	function toggle(surface: ApiBaseUrlSurface, on: boolean) {
		const next = allSurfaces.filter((s) => (s === surface ? on : appliesTo.includes(s)))
		// Ticking every box returns to unset, so a surface added later follows the url too.
		$values['api_base_url_applies_to'] = next.length === allSurfaces.length ? null : next
	}
</script>

<div class="flex flex-col gap-2">
	<TextInput
		inputProps={{ type: 'text', id: 'api_base_url', placeholder, disabled }}
		bind:value={$values['api_base_url']}
		class="max-w-lg"
	/>
	<div class="flex flex-col gap-1">
		<span class="text-xs text-secondary">Applies to</span>
		{#each API_BASE_URL_SURFACES as { value, label } (value)}
			<label class="flex items-center gap-2 text-xs">
				<!-- The form stores an empty list as unset, which means every surface: to apply
				it to none, clear the url instead. -->
				<Checkbox
					checked={appliesTo.includes(value)}
					disabled={disabled || (appliesTo.length === 1 && appliesTo[0] === value)}
					onChange={() => toggle(value, !appliesTo.includes(value))}
				/>
				<span>{label}</span>
			</label>
		{/each}
	</div>
</div>
