<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import { Maximize2, Minimize2, Settings, StickyNote, Ungroup } from 'lucide-svelte'
	import { NoteColor, NOTE_COLOR_SWATCHES } from './noteColors'
	import Popover from '../meltComponents/Popover.svelte'
	import { Tooltip } from '../meltComponents'
	import Toggle from '../Toggle.svelte'

	interface Props {
		note: string | undefined | null
		color: string | undefined
		collapsedByDefault: boolean
		collapsed: boolean
		settingsOpen?: boolean
		onAddNote: () => void
		onRemoveNote: () => void
		onToggleCollapse: () => void
		onUpdateColor: (color: NoteColor) => void
		onUpdateCollapsedDefault: (value: boolean) => void
		onDeleteGroup?: () => void
	}

	let {
		note,
		color,
		collapsedByDefault,
		collapsed,
		settingsOpen = $bindable(false),
		onAddNote,
		onRemoveNote,
		onToggleCollapse,
		onUpdateColor,
		onUpdateCollapsedDefault,
		onDeleteGroup = undefined
	}: Props = $props()
</script>

<div class="absolute -translate-y-[100%] top-2 right-0 h-7 p-1 flex flex-row gap-1">
	{#if note == null}
		<Tooltip>
			<button
				class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1 rounded-md"
				onclick={stopPropagation(preventDefault(onAddNote))}
				onpointerdown={stopPropagation(preventDefault(() => {}))}
			>
				<StickyNote size={12} />
			</button>
			<svelte:fragment slot="text">Add note</svelte:fragment>
		</Tooltip>
	{:else}
		<Tooltip>
			<button
				class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-red-400 hover:text-white p-1 rounded-md"
				onclick={stopPropagation(preventDefault(onRemoveNote))}
				onpointerdown={stopPropagation(preventDefault(() => {}))}
			>
				<StickyNote size={12} />
			</button>
			<svelte:fragment slot="text">Remove note</svelte:fragment>
		</Tooltip>
	{/if}
	<Tooltip>
		<button
			class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1 rounded-md"
			onclick={stopPropagation(preventDefault(onToggleCollapse))}
			onpointerdown={stopPropagation(preventDefault(() => {}))}
		>
			{#if collapsed}
				<Maximize2 size={12} />
			{:else}
				<Minimize2 size={12} />
			{/if}
		</button>
		<svelte:fragment slot="text">{collapsed ? 'Expand' : 'Collapse'} group</svelte:fragment>
	</Tooltip>
	<Popover
		placement="bottom"
		contentClasses="p-4"
		floatingConfig={{ strategy: 'absolute' }}
		usePointerDownOutside
		bind:isOpen={settingsOpen}
	>
		{#snippet trigger()}
			<Tooltip>
				<button
					class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1 rounded-md"
				>
					<Settings size={12} />
				</button>
				<svelte:fragment slot="text">Group settings</svelte:fragment>
			</Tooltip>
		{/snippet}
		{#snippet content()}
			<div class="grid grid-cols-5 gap-1" style="min-width: 140px">
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
			<div class="border-t mt-2 pt-2 flex flex-col gap-2">
				<Toggle
					size="xs"
					checked={collapsedByDefault}
					options={{ right: 'Collapsed by default' }}
					on:change={(e) => onUpdateCollapsedDefault(e.detail)}
				/>
			</div>
		{/snippet}
	</Popover>
	{#if onDeleteGroup}
		<Tooltip>
			<button
				class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-red-400 hover:text-white p-1 rounded-md"
				onclick={stopPropagation(preventDefault(onDeleteGroup))}
				onpointerdown={stopPropagation(preventDefault(() => {}))}
			>
				<Ungroup size={12} />
			</button>
			<svelte:fragment slot="text">Ungroup</svelte:fragment>
		</Tooltip>
	{/if}
</div>
