<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { Code2, Loader2, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { importScriptStore } from './scriptStore.svelte'
	import YAML from 'yaml'

	let { aiId, aiDescription }: { aiId: string; aiDescription: string } = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let pendingRaw: string | undefined = $state(undefined)
	let importType: 'yaml' | 'json' = $state('yaml')

	async function importRaw() {
		const parsed =
			importType === 'yaml' ? YAML.parse(pendingRaw ?? '') : JSON.parse(pendingRaw ?? '')
		$importScriptStore = parsed
		await goto(`${base}/scripts/add?import=true`)
		drawer?.closeDrawer?.()
	}
</script>

<div class="flex flex-row gap-2">
	<Button
		id="create-script-button"
		{aiId}
		{aiDescription}
		unifiedSize="lg"
		variant="accent"
		startIcon={{ icon: Plus }}
		href="{base}/scripts/add"
		endIcon={{ icon: Code2 }}
		dropdownItems={[
			{
				label: 'Import from YAML/JSON',
				onClick: () => {
					drawer?.toggleDrawer?.()
				}
			},
			{
				label: 'Workflow-as-Code in Python',
				onClick: () => {
					goto(`${base}/scripts/add?nodraft=true&wac=python`)
				}
			},
			{
				label: 'Workflow-as-Code in TypeScript',
				onClick: () => {
					goto(`${base}/scripts/add?nodraft=true&wac=typescript`)
				}
			}
		]}
	>
		Script
	</Button>
</div>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title="Import script from YAML/JSON"
		on:close={() => drawer?.toggleDrawer?.()}
	>
		<Tabs bind:selected={importType}>
			<Tab value="yaml" label="YAML" />
			<Tab value="json" label="JSON" />
			{#snippet content()}
				<div class="relative pt-2 h-full">
					{#key importType}
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
					{/key}
				</div>
			{/snippet}
		</Tabs>
		{#snippet actions()}
			<Button size="sm" on:click={importRaw}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
