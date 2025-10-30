<script lang="ts">
	import { Pen } from 'lucide-svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		value: string;
		disabled?: boolean;
	}

	let { value = $bindable(), disabled = false }: Props = $props();

	function blur(e: KeyboardEvent) {
		e.key === 'Enter' && (e?.target as any)?.blur()
	}
</script>

<div class="flex flex-row center-center gap-1 min-w-32 lg:min-w-64 w-full max-w-md">
	<div class="relative w-full flex">
		<TextInput
			inputProps={{
				title: 'Rename',
				type: 'text',
				placeholder: 'Untitled',
				disabled,
				onkeydown: blur
			}}
			class="w-full font-semibold"
			bind:value
		/>
		{#if !disabled}
			<Pen class="absolute top-2 right-2 pen-icon -z-10 opacity-60" size={14} />
		{/if}
	</div>
</div>
