<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import { EllipsisVertical, StickyNote, Ungroup } from 'lucide-svelte'
	import { NoteColor, NOTE_COLOR_LIST, NOTE_COLOR_SWATCHES } from './noteColors'
	import Toggle from '../Toggle.svelte'
	import ColorSwatchGrid from '../common/colorPicker/ColorSwatchGrid.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { twMerge } from 'tailwind-merge'
	import MoveHandleButton from './MoveHandleButton.svelte'
	import type { MoveManager } from './moveManager.svelte'

	interface Props {
		note: string | undefined | null
		color: string | undefined
		autocollapse: boolean
		visible?: boolean
		menuOpen?: boolean
		moveManager?: MoveManager
		moveModuleId?: string
		onMenuOpenChange?: (open: boolean) => void
		onAddNote: () => void
		onRemoveNote: () => void
		onUpdateColor: (color: NoteColor) => void
		onUpdateAutocollapse: (value: boolean) => void
		onDeleteGroup?: () => void
	}

	let {
		note,
		color,
		autocollapse,
		visible = true,
		menuOpen = $bindable(),
		moveManager,
		moveModuleId,
		onMenuOpenChange,
		onAddNote,
		onRemoveNote,
		onUpdateColor,
		onUpdateAutocollapse,
		onDeleteGroup = undefined
	}: Props = $props()

	$effect(() => {
		onMenuOpenChange?.(menuOpen ?? false)
	})
</script>

<div
	class="absolute -translate-y-[100%] top-2 right-0 h-7 p-1 min-w-7 flex flex-row gap-2"
	style="will-change: transform;"
>
	{#if moveManager && moveModuleId}
		<MoveHandleButton
			{moveManager}
			moduleId={moveModuleId}
			singleNode
			{visible}
			onClickMove={() => moveManager.toggleMoving(moveModuleId!)}
		/>
	{/if}
	{#if note == null}
		<button
			class={twMerge(
				'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
				visible ? 'block' : '!hidden',
				'shadow-md rounded-md'
			)}
			onpointerdown={stopPropagation(preventDefault(() => {}))}
			onclick={() => onAddNote()}
			title="Add note"
		>
			<StickyNote size={12} />
		</button>
	{/if}
	<DropdownV2
		placement="bottom-end"
		bind:open={menuOpen}
		fixedHeight={false}
		usePointerDownOutside
		customMenu
	>
		{#snippet buttonReplacement()}
			<button
				class={twMerge(
					'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
					visible || menuOpen ? 'block' : '!hidden',
					'shadow-md rounded-md'
				)}
				onpointerdown={stopPropagation(preventDefault(() => {}))}
				title="Actions"
			>
				<EllipsisVertical size={12} />
			</button>
		{/snippet}
		{#snippet menu()}
			<div
				class="bg-surface-tertiary dark:border w-56 origin-top-right rounded-lg shadow-lg focus:outline-none py-1"
			>
				<!-- Color picker -->
				<div class="px-4 py-2">
					<ColorSwatchGrid
						colors={NOTE_COLOR_LIST}
						swatches={NOTE_COLOR_SWATCHES}
						selected={color ?? NoteColor.BLUE}
						onSelect={onUpdateColor}
					/>
				</div>

				<!-- Autocollapse toggle -->
				<div class="px-4 py-2">
					<Toggle
						size="xs"
						checked={autocollapse}
						options={{ right: 'Autocollapse' }}
						on:change={(e) => onUpdateAutocollapse(e.detail)}
					/>
				</div>

				<div class="my-1 border-t border-border-light"></div>

				<!-- Add / Remove note -->
				<button
					class="px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs w-full flex flex-row gap-2 items-center rounded-sm"
					onclick={() => {
						note == null ? onAddNote() : onRemoveNote()
						menuOpen = false
					}}
				>
					<StickyNote size={14} class="shrink-0" />
					<p class="truncate grow min-w-0 whitespace-nowrap text-left"
						>{note == null ? 'Add note' : 'Remove note'}</p
					>
				</button>

				{#if onDeleteGroup}
					<div class="my-1 border-t border-border-light"></div>

					<!-- Ungroup -->
					<button
						class="px-4 py-2 font-normal hover:bg-red-500/10 cursor-pointer text-xs w-full flex flex-row gap-2 items-center rounded-sm text-red-600 dark:text-red-400"
						onclick={() => {
							onDeleteGroup?.()
							menuOpen = false
						}}
					>
						<Ungroup size={14} class="shrink-0" />
						<p class="truncate grow min-w-0 whitespace-nowrap text-left">Ungroup</p>
					</button>
				{/if}
			</div>
		{/snippet}
	</DropdownV2>
</div>
