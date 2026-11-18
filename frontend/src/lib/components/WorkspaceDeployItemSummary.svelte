<script lang="ts">
	import ExternalEditLink from './ExternalEditLink.svelte'

	interface Props {
		/** Path of the item, used as the fallback label and link title. */
		path: string
		/** Editor URL; when set the summary becomes a new-tab link. */
		editUrl?: string
		/** Deployed/source-side display name (struck through when renamed). */
		oldSummary?: string
		/** Draft/target-side display name (the surviving name when renamed). */
		newSummary?: string
		/** Render `~~oldSummary~~ newSummary`. The caller decides this from its
		 * own concepts (fork: exists-in-both & selectable; draft: !draft_only),
		 * keeping page-specific logic out of this presentational component. */
		renamed: boolean
	}

	let { path, editUrl, oldSummary, newSummary, renamed }: Props = $props()
</script>

{#snippet label()}
	{#if renamed}
		<!-- Two names side by side: don't truncate, mirror the fork compare page. -->
		<span class="line-through text-secondary">{oldSummary || path}</span>
		{newSummary || path}
	{:else}
		<span class="truncate">{newSummary || oldSummary || path}</span>
	{/if}
{/snippet}

{#if editUrl}
	<!-- Truncate the single-name case; let a rename pair render full-width. -->
	<ExternalEditLink
		href={editUrl}
		title="Open {path} in a new tab"
		class={renamed ? 'text-emphasis' : 'text-emphasis truncate'}
	>
		{@render label()}
	</ExternalEditLink>
{:else}
	{@render label()}
{/if}
