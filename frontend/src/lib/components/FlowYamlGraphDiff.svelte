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

<Tabs bind:selected={diffMode}>
	<Tab value="yaml" label="YAML" />
	<Tab value="graph" label="Graph" />
</Tabs>
{#if diffMode === 'yaml'}
	{#await import('$lib/components/DiffEditor.svelte')}
		<Loader2 class="animate-spin" />
	{:then Module}
		<Module.default
			open={true}
			automaticLayout
			className="h-full"
			defaultLang="yaml"
			defaultOriginal={beforeYaml}
			defaultModified={afterYaml}
			readOnly
		/>
	{/await}
{:else if diffMode === 'graph'}
	{#await import('$lib/components/FlowGraphDiffViewer.svelte')}
		<Loader2 class="animate-spin" />
	{:then Module}
		<Module.default {beforeYaml} {afterYaml} />
	{/await}
{/if}
