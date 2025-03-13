<script lang="ts">
	import { Popover } from '$lib/components/meltComponents'
	import { X } from 'lucide-svelte'
	import { ContextIconMap } from './core'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import { twMerge } from 'tailwind-merge'

	export let kind: string
	export let error: string | undefined = undefined
	export let deletable = false

	const icon = ContextIconMap[kind]
	let showDelete = false
</script>

<Popover disablePopup={kind !== 'error' || error === undefined} openOnHover>
	<svelte:fragment slot="trigger">
		<div
			class={twMerge(
				'border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-tertiary text-xs cursor-default',
				deletable ? 'hover:cursor-pointer' : ''
			)}
			on:mouseenter={() => (showDelete = true)}
			on:mouseleave={() => (showDelete = false)}
			aria-label="Context element"
			role="button"
			tabindex={0}
		>
			<button on:click class:cursor-default={!deletable}>
				{#if showDelete && deletable}
					<X size={16} />
				{:else}
					<svelte:component this={icon} size={16} />
				{/if}
			</button>
			{kind}
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="p-2 max-w-96 max-h-[300px] text-xs whitespace-break-spaces overflow-auto">
			<Highlight language={json} code={error} class="w-full" />
		</div>
	</svelte:fragment>
</Popover>
