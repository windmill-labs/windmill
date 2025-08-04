<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'

	interface Props {
		label?: string | undefined
		primary?: boolean
		disabled?: boolean
		headless?: boolean
		required?: boolean
		headerClass?: string
		class?: string | undefined
		header?: import('svelte').Snippet
		error?: import('svelte').Snippet
		action?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		label = undefined,
		primary = false,
		disabled = false,
		headless = false,
		required = false,
		headerClass = '',
		class: clazz = undefined,
		header,
		error,
		action,
		children
	}: Props = $props()
</script>

<div class={twMerge(disabled ? 'opacity-60 pointer-events-none' : '', clazz)}>
	<div class="flex flex-row justify-between items-center w-full">
		{#if !headless}
			<div class={twMerge('flex flex-row items-center gap-2', headerClass)}>
				<span
					class="{primary ? 'text-primary' : 'text-secondary'} text-sm leading-6 whitespace-nowrap"
					>{label}
					{#if required}
						<Required required={true} />
					{/if}
				</span>
				{@render header?.()}
			</div>
		{/if}
		{@render error?.()}
		{@render action?.()}
	</div>
	{@render children?.()}
</div>
