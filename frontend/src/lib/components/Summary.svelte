<script lang="ts">
	import { Pen } from 'lucide-svelte'

	export let value: string

	function blur(e: KeyboardEvent) {
		e.key === 'Enter' && (e?.target as any)?.blur()
	}
</script>

<div class="flex flex-row center-center gap-1 min-w-32 lg:min-w-64 w-full max-w-md">
	<div class="relative w-full">
		<input
			title="Rename"
			type="text"
			placeholder="Untitled"
			class="windmillapp app-title text-sm w-full font-semibold"
			bind:value
			on:keydown={blur}
		/>
		<Pen class="absolute top-2 right-2 pen-icon -z-10 opacity-60" size={14} />
	</div>
</div>

<style lang="postcss" global>
	.app-title {
		@apply block text-sm w-full p-1 px-2 rounded border-0 font-semibold;
	}

	.app-title:hover {
		@apply border-gray-300 shadow-sm;
	}

	.app-title:focus {
		@apply border-gray-300  ring-frost-100 ring-offset-0 ring-1;
		box-shadow: var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000);
	}

	.dark .app-title {
		@apply bg-gray-700 text-gray-200 border-gray-600;
	}

	.dark .app-title:focus {
		@apply ring-frost-700;
	}

	.app-title:focus + .pen-icon {
		@apply hidden;
	}
</style>
