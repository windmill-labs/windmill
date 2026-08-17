<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'

	interface Props {
		beforeYaml: string
		afterYaml: string
		/** Side-by-side vs unified. Leave undefined to let
		 * FlowGraphDiffViewer show its own user-facing toggle (matches the
		 * pre-fork-diff-drawer behavior). */
		inlineDiff?: boolean
		/** Forward Monaco's auto-inline opt-out to the YAML-mode DiffEditor. */
		disableAutoInline?: boolean
		/** Forwarded to FlowGraphDiffViewer — render an empty surface
		 * placeholder for the "before" / "after" pane when the item is
		 * added / removed. */
		beforeMissing?: boolean
		afterMissing?: boolean
	}

	let {
		beforeYaml,
		afterYaml,
		inlineDiff = undefined,
		disableAutoInline = false,
		beforeMissing = false,
		afterMissing = false
	}: Props = $props()
	let diffMode: 'yaml' | 'graph' = $state('graph')
</script>

<div class="flex flex-col h-full min-h-[500px]">
	<Tabs bind:selected={diffMode}>
		<Tab value="graph" label="Graph" />
		<Tab value="yaml" label="YAML" />
	</Tabs>

	<div class="flex-1 min-h-0">
		{#if diffMode === 'yaml'}
			{#await import('$lib/components/DiffEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					open={true}
					automaticLayout
					className="h-full min-h-[400px]"
					defaultLang="yaml"
					defaultOriginal={beforeYaml}
					defaultModified={afterYaml}
					{inlineDiff}
					{disableAutoInline}
					readOnly
				/>
			{/await}
		{:else}
			{#await import('$lib/components/FlowGraphDiffViewer.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default {beforeYaml} {afterYaml} {beforeMissing} {afterMissing} {inlineDiff} />
			{/await}
		{/if}
	</div>
</div>
