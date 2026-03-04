<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import { EllipsisVertical, StickyNote, Ungroup } from 'lucide-svelte'
	import { NoteColor, NOTE_COLOR_SWATCHES } from './noteColors'
	import Toggle from '../Toggle.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		note: string | undefined | null
		color: string | undefined
		collapsedByDefault: boolean
		visible?: boolean
		menuOpen?: boolean
		onAddNote: () => void
		onRemoveNote: () => void
		onUpdateColor: (color: NoteColor) => void
		onUpdateCollapsedDefault: (value: boolean) => void
		onDeleteGroup?: () => void
	}

	let {
		note,
		color,
		collapsedByDefault,
		visible = true,
		menuOpen = $bindable(false),
		onAddNote,
		onRemoveNote,
		onUpdateColor,
		onUpdateCollapsedDefault,
		onDeleteGroup = undefined
	}: Props = $props()
</script>

<div
	class="absolute -translate-y-[100%] top-2 right-0 h-7 p-1 min-w-7"
	style="will-change: transform;"
>
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
			<div class="bg-surface-tertiary dark:border w-56 origin-top-right rounded-lg shadow-lg focus:outline-none py-1">
				<!-- Color picker -->
				<div class="px-4 py-2">
					<div class="grid grid-cols-5 gap-1">
						{#each Object.values(NoteColor) as c (c)}
							<button
								class="w-6 h-6 rounded-full hover:scale-110 transition-transform duration-100
									{NOTE_COLOR_SWATCHES[c]}
									{(color ?? NoteColor.BLUE) === c ? 'ring-2 ring-accent' : 'dark:border-gray-600'}"
								onclick={() => onUpdateColor(c)}
								title={c.charAt(0).toUpperCase() + c.slice(1)}
							></button>
						{/each}
					</div>
				</div>

				<!-- Collapsed by default toggle -->
				<div class="px-4 py-2">
					<Toggle
						size="xs"
						checked={collapsedByDefault}
						options={{ right: 'Collapsed by default' }}
						on:change={(e) => onUpdateCollapsedDefault(e.detail)}
					/>
				</div>

				<div class="my-1 border-t border-border-light"></div>

				<!-- Add / Remove note -->
				<button
					class="px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs w-full flex flex-row gap-2 items-center rounded-sm"
					onclick={() => { note == null ? onAddNote() : onRemoveNote(); menuOpen = false }}
				>
					<StickyNote size={14} class="shrink-0" />
					<p class="truncate grow min-w-0 whitespace-nowrap text-left">{note == null ? 'Add note' : 'Remove note'}</p>
				</button>

				{#if onDeleteGroup}
					<div class="my-1 border-t border-border-light"></div>

					<!-- Ungroup -->
					<button
						class="px-4 py-2 font-normal hover:bg-red-500/10 cursor-pointer text-xs w-full flex flex-row gap-2 items-center rounded-sm text-red-600 dark:text-red-400"
						onclick={() => { onDeleteGroup?.(); menuOpen = false }}
					>
						<Ungroup size={14} class="shrink-0" />
						<p class="truncate grow min-w-0 whitespace-nowrap text-left">Ungroup</p>
					</button>
				{/if}
			</div>
		{/snippet}
	</DropdownV2>
</div>
