<script lang="ts" module>
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'

	const HEIGHT_SETTING = 'home-composer-height:'
	// The box's 4px top margin and the 8px under the block.
	const BLOCK_MARGINS = 12
	const ONE_LINE_HEIGHT = 48 + BLOCK_MARGINS
	// The suggestion chips' row under an open box. A disabled composer has none, and its box,
	// the shortest open one, is 78px.
	const SUGGESTIONS_ROW = 36
	const OPEN_BOX_MIN = 78

	// The composer rests open beside the session list and on one line beside the workspace nav,
	// so each side of the sidebar switch keeps its own height.
	function heightSetting(workspace: string, sessionsListed: boolean): string {
		return HEIGHT_SETTING + workspace + (sessionsListed ? ':sessions' : '')
	}

	/** The height Home's composer took when it last appeared in this workspace, beside this list. */
	export function composerHeight(
		workspace: string | undefined,
		sessionsListed: boolean
	): number | undefined {
		const stored = workspace
			? Number(getLocalSetting(heightSetting(workspace, sessionsListed)))
			: NaN
		return Number.isFinite(stored) && stored > 0 ? stored : undefined
	}

	export function rememberComposerHeight(
		workspace: string | undefined,
		sessionsListed: boolean,
		height: number
	) {
		if (workspace && height > 0)
			storeLocalSetting(heightSetting(workspace, sessionsListed), String(height))
	}
</script>

<script lang="ts">
	import { ArrowUp, MessagesSquare } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Button from '../common/button/Button.svelte'
	import {
		composerBoxClass,
		HERO_BOX_CLASS,
		STARTER_PLACEHOLDER
	} from '../copilot/chat/composerBox'

	// Stands in for the composer while its code and config load, drawn as the composer it
	// replaces: the same box, mark and placeholder, in the shape it last took in this workspace.
	// Without a remembered height it is the one-line shape, which is what a workspace past a
	// handful of items rests in.
	let { height }: { height?: number } = $props()

	const open = $derived(height !== undefined && height > ONE_LINE_HEIGHT)
	const suggestions = $derived(open && height! - BLOCK_MARGINS - SUGGESTIONS_ROW >= OPEN_BOX_MIN)
	const boxClass = twMerge(composerBoxClass(), HERO_BOX_CLASS)
</script>

{#snippet bar(width: string)}
	<span class="animate-skeleton dark:bg-frost-900/50 rounded h-3 {width}"></span>
{/snippet}

{#snippet sendButton()}
	<Button variant="subtle" unifiedSize="sm" iconOnly disabled startIcon={{ icon: ArrowUp }} />
{/snippet}

<div
	class="w-full pb-2 flex flex-col"
	style:height={open ? `${height}px` : undefined}
	aria-busy="true"
	aria-label="Loading the AI composer"
>
	<!-- The placeholder's `-px` offsets stand for the real field's transparent 1px border. -->
	{#if open}
		<div class="mt-1 flex-1 min-h-0 flex flex-col {boxClass}">
			<div class="flex items-start">
				<span class="flex items-center h-5 mt-[13px] ml-4 shrink-0">
					<MessagesSquare size={16} class="text-ai" />
				</span>
				<span class="mt-px ml-px pl-2 pr-4 pt-3 text-xs leading-5 text-gray-500 truncate">
					{STARTER_PLACEHOLDER}
				</span>
			</div>
			<div class="mt-auto flex flex-row items-center gap-1.5 px-3 pb-1.5">
				<div class="flex-1 min-w-0 flex flex-row items-center gap-3">
					{@render bar('w-24')}
					{@render bar('w-20')}
				</div>
				{@render bar('w-12')}
				{@render sendButton()}
			</div>
		</div>
		{#if suggestions}
			<div class="pt-2 flex flex-row items-center gap-1.5 shrink-0">
				{#each ['w-20', 'w-24', 'w-24'] as width, i (i)}
					<span class="h-7 px-3 flex items-center rounded-full border border-border-light">
						{@render bar(width)}
					</span>
				{/each}
			</div>
		{/if}
	{:else}
		<div class="mt-1 h-12 flex flex-row items-center {boxClass}">
			<span class="flex items-center ml-4 shrink-0">
				<MessagesSquare size={16} class="text-ai" />
			</span>
			<span class="flex-1 min-w-0 ml-px pl-2 pr-2 text-xs leading-5 text-gray-500 truncate">
				{STARTER_PLACEHOLDER}
			</span>
			<div class="flex flex-row items-center gap-1 pr-3 shrink-0">
				{@render bar('w-12')}
				{@render sendButton()}
			</div>
		</div>
	{/if}
</div>
