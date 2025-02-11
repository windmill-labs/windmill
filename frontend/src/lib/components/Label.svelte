<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'

	export let label: string | undefined = undefined
	export let primary = false
	export let disabled = false
	export let headless = false
	export let required = false
</script>

<div class={twMerge(disabled ? 'opacity-60 pointer-events-none' : '', $$props.class)}>
	<div class="flex flex-row justify-between items-center w-full">
		{#if !headless}
			<div class="flex flex-row items-center gap-2">
				<span class="{primary ? 'text-primary' : 'text-secondary'} text-sm leading-6"
					>{label}
					{#if required}
						<Required required={true} />
					{/if}
				</span>
				<slot name="header" />
			</div>
		{/if}
		<slot name="error" />
		<slot name="action" />
	</div>
	<slot />
</div>
