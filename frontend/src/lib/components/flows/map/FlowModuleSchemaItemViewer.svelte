<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { Pencil } from 'lucide-svelte'

	let iconWidth: number = 0
	let idBadgeWidth: number | undefined = undefined
	export let label: string = ''
	export let path: string = ''
	export let id: string = ''
	export let deletable: boolean = false
	export let bold: boolean = false
	export let editId: boolean = false
	export let hover: boolean = false

	$: marginLeft = Math.max(iconWidth ?? 0, idBadgeWidth ?? 0) * 2 + 32
</script>

<div
	class="relative flex gap-1 justify-between items-center w-full overflow-hidden rounded-sm
	 p-2 text-2xs module text-primary"
>
	{#if $$slots.icon && true}
		<div class="flex-none" bind:clientWidth={iconWidth}>
			<slot name="icon" />
		</div>
	{/if}

	<Popover
		class="absolute left-1/2 transform -translate-x-1/2 center-center"
		style="max-width: calc(100% - {marginLeft}px)"
	>
		<div class="text-center truncate {bold ? '!font-bold' : 'font-normal'}">
			{label}
		</div>
		<svelte:fragment slot="text">
			<div>
				<div>{label}</div>
				{#if path != ''}<div>{path}</div>{/if}
			</div>
		</svelte:fragment>
	</Popover>

	<div class="flex items-center space-x-2 relative max-w-[25%]" bind:clientWidth={idBadgeWidth}>
		{#if id && id !== 'preprocessor' && !id.startsWith('failure') && !id.startsWith('subflow:')}
			<Badge
				color="indigo"
				wrapperClass="max-w-full"
				baseClass="max-w-full truncate !px-1"
				title={id}
			>
				<span class="max-w-full text-2xs truncate">{id}</span></Badge
			>
			{#if deletable}
				<button
					class="absolute -left-[28px] z-10 h-[20px] rounded-l rounded-t rounded-s w-[20px] trash center-center text-secondary bg-surface duration-0 hover:bg-blue-400 {editId
						? '!bg-blue-400'
						: ''} hover:text-white
hover:border-blue-700 hover:!visible {hover ? '' : '!hidden'}"
					on:click|preventDefault|stopPropagation={(event) => (editId = !editId)}
					title="Edit Id"><Pencil size={14} /></button
				>
			{/if}
		{:else if id?.startsWith('subflow:')}
			<Badge color="blue" wrapperClass="max-w-full" baseClass="!px-1" title={id}>
				<span class="max-w-full text-2xs truncate">{id.substring('subflow:'.length)}</span></Badge
			>
		{/if}
	</div>
</div>
