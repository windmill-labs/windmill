<script lang="ts">
	import type { EditorConfig, StaticInput } from '../../types'
	import Toggle from '$lib/components/Toggle.svelte'

	export let input: StaticInput
</script>

<Toggle bind:checked={input.visible} options={{ right: 'Visible' }} />
<input bind:value={input.value} />
