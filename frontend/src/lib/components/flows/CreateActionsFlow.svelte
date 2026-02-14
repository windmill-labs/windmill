<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
	import { Loader2, Plus } from 'lucide-svelte'
	import YAML from 'yaml'
	let drawer: Drawer | undefined = $state(undefined)
	let pendingRaw: string | undefined = $state(undefined)
	let importType: 'yaml' | 'json' = $state('yaml')

	async function importRaw() {
		$importFlowStore =
			importType === 'yaml' ? YAML.parse(pendingRaw ?? '') : JSON.parse(pendingRaw ?? '')
		await goto('/flows/add')
		drawer?.closeDrawer?.()
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
		<Button
			id="create-flow-button"
			aiId="flows-create-actions-flow"
			aiDescription="Create a new flow"
			unifiedSize="lg"
			startIcon={{ icon: Plus }}
			endIcon={{ icon: BarsStaggered }}
			href="{base}/flows/add?nodraft=true"
			variant="accent"
			dropdownItems={[
				{
					label: 'Import from YAML',
					onClick: () => {
						drawer?.toggleDrawer?.()
						importType = 'yaml'
					}
				},
				{
					label: 'Import from JSON',
					onClick: () => {
						drawer?.toggleDrawer?.()
						importType = 'json'
					}
				}
			]}
		>
			Flow
		</Button>
	</div>

<!-- Raw JSON -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title={'Import flow from ' + (importType === 'yaml' ? 'YAML' : 'JSON')}
		on:close={() => drawer?.toggleDrawer?.()}
	>
		{#await import('$lib/components/SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:code={pendingRaw}
				lang={importType}
				class="h-full"
				fixedOverflowWidgets={false}
			/>
		{/await}
		{#snippet actions()}
			<Button size="sm" on:click={importRaw}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
