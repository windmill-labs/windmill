<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { LayoutDashboard, Loader2, Plus } from 'lucide-svelte'
	import { importStore } from '../apps/store'

	import YAML from 'yaml'

	let drawer: Drawer | undefined = undefined
	let pendingRaw: string = ''

	let importType: 'yaml' | 'json' = 'yaml'

	async function importRaw() {
		$importStore = importType === 'yaml' ? YAML.parse(pendingRaw) : JSON.parse(pendingRaw)
		await goto('/apps/add?nodraft=true')
		drawer?.closeDrawer?.()
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<Button
		aiId="apps-create-actions-app"
		aiDescription="Create a new low-code app"
		size="sm"
		spacingSize="xl"
		startIcon={{ icon: Plus }}
		href="{base}/apps/add?nodraft=true"
		color="marine"
		dropdownItems={[
			{
				label: 'Import low-code app from YAML',
				onClick: () => {
					drawer?.toggleDrawer?.()
					importType = 'yaml'
				}
			},
			{
				label: 'Import low-code app from JSON',
				onClick: () => {
					drawer?.toggleDrawer?.()
					importType = 'json'
				}
			}
			// {
			// 	label: 'Build app in React/Vue/Svelte (alpha)',
			// 	onClick: () => goto('/apps_raw/add?nodraft=true')
			// }
		]}
	>
		<div class="flex flex-row items-center">
			App <LayoutDashboard class="ml-1.5" size={18} />
		</div>
	</Button>
</div>

<!-- Raw JSON -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title={'Import low-code app from ' + (importType === 'yaml' ? 'YAML' : 'JSON')}
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
