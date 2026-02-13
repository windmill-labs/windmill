<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Ellipsis, Pencil } from 'lucide-svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Runnable } from '../apps/inputType'
	import TextInput from '../text_input/TextInput.svelte'
	import { tick } from 'svelte'

	interface Props {
		id: string
		runnable: Runnable
		isSelected: boolean
		isEditing: boolean
		onSelect: () => void
		onDelete: () => void
		onRename: (newId: string) => void
		onRequestEdit: () => void
		onCancelEdit: () => void
	}

	let {
		id,
		runnable,
		isSelected,
		isEditing,
		onSelect,
		onDelete,
		onRename,
		onRequestEdit,
		onCancelEdit
	}: Props = $props()

	let dropdownOpen = $state(false)
	let editValue = $state(id)
	let textInputElement: TextInput | undefined = $state()

	function finishEdit() {
		const trimmed = editValue.trim()
		if (trimmed && trimmed !== id) {
			onRename(trimmed)
		} else {
			onCancelEdit()
		}
	}

	function handleInputKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			finishEdit()
		} else if (e.key === 'Escape') {
			editValue = id
			onCancelEdit()
		}
	}

	$effect(() => {
		if (isEditing && textInputElement) {
			editValue = id
			tick().then(() => {
				textInputElement?.focus()
				textInputElement?.select()
			})
		}
	})
</script>

{#if isEditing}
	<div
		class="w-full flex items-center gap-1 h-6 rounded-md px-1
		{isSelected ? 'bg-surface-accent-selected text-accent' : ''}"
	>
		<TextInput
			bind:this={textInputElement}
			bind:value={editValue}
			inputProps={{
				onkeydown: handleInputKeydown,
				onblur: () => finishEdit(),
				type: 'text'
			}}
			size="xs"
		/>
	</div>
{:else}
	<button
		{id}
		class="relative w-full gap-1 flex items-center h-6 rounded-md px-1 group
		{isSelected ? 'bg-surface-accent-selected text-accent' : 'hover:bg-surface-hover'}"
		onclick={onSelect}
	>
		<Badge color="indigo" class={isSelected ? 'bg-surface-tertiary' : ''}>{id}</Badge>
		<span class="text-xs truncate font-normal pr-6">{runnable?.name}</span>
		<DropdownV2
			items={[
				{
					displayName: 'Rename',
					icon: Pencil,
					action: () => onRequestEdit()
				},
				{
					displayName: 'Delete',
					action: onDelete,
					type: 'delete'
				}
			]}
			class={twMerge(
				'absolute -translate-y-1/2 top-1/2 right-1 hidden group-hover:block',
				dropdownOpen ? 'block' : ''
			)}
			bind:open={dropdownOpen}
		>
			{#snippet buttonReplacement()}
				<Button
					iconOnly
					unifiedSize="xs"
					variant="subtle"
					startIcon={{ icon: Ellipsis }}
					nonCaptureEvent
				/>
			{/snippet}
		</DropdownV2>
	</button>
{/if}
