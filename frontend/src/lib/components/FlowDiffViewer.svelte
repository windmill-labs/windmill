<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'

	interface Props {
		beforeYaml: string
		afterYaml: string
	}

	let { beforeYaml, afterYaml }: Props = $props()
	let diffMode: 'yaml' | 'graph' = $state('yaml')
</script>

<div class="flex flex-col h-full min-h-[500px] gap-2">
	<Tabs bind:selected={diffMode}>
		<Tab value="yaml" label="YAML" />
		<Tab value="graph" label="Graph" />
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
					readOnly
				/>
			{/await}
		{:else}
			{#await import('$lib/components/FlowGraphDiffViewer.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default {beforeYaml} {afterYaml} />
			{/await}
		{/if}
	</div>
</div>
