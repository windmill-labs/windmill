<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { Item } from '$lib/utils'
	import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
	import { importStore } from '$lib/components/apps/store'
	import { HOME_SHOW_CREATE_FLOW, HOME_SHOW_CREATE_APP } from '$lib/consts'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Code2, LayoutDashboard, Loader2, Plus, Download, ChevronDown } from 'lucide-svelte'
	import YAML from 'yaml'

	// Import drawer state — shared across flow / WAC / app import flows.
	let flowDrawer: Drawer | undefined = $state(undefined)
	let wacDrawer: Drawer | undefined = $state(undefined)
	let appDrawer: Drawer | undefined = $state(undefined)
	let pendingFlowRaw: string = $state('')
	let pendingWacRaw: string = $state('')
	let pendingAppRaw: string = $state('')
	let flowImportType: 'yaml' | 'json' = $state('yaml')
	let wacImportType: 'yaml' | 'json' = $state('yaml')
	let appImportType: 'yaml' | 'json' = $state('yaml')
	let appKind: 'lowcode' | 'fullcode' = $state('lowcode')

	async function importFlowRaw() {
		$importFlowStore =
			flowImportType === 'yaml' ? YAML.parse(pendingFlowRaw) : JSON.parse(pendingFlowRaw)
		await goto(`${base}/flows/add`)
		flowDrawer?.closeDrawer?.()
	}

	async function importWacRaw() {
		$importScriptStore =
			wacImportType === 'yaml' ? YAML.parse(pendingWacRaw) : JSON.parse(pendingWacRaw)
		await goto(`${base}/scripts/add?import=true`)
		wacDrawer?.closeDrawer?.()
	}

	async function importAppRaw() {
		const parsed = appImportType === 'yaml' ? YAML.parse(pendingAppRaw) : JSON.parse(pendingAppRaw)
		if (appKind === 'fullcode') {
			// /apps_raw/add triggers a full page reload (cross-origin isolation),
			// so the in-memory importStore would be lost — use sessionStorage.
			sessionStorage.setItem('rawAppImport', JSON.stringify(parsed))
			await goto(`${base}/apps_raw/add`)
		} else {
			$importStore = parsed
			await goto(`${base}/apps/add`)
		}
		appDrawer?.closeDrawer?.()
	}

	let newItems: Item[] = $derived([
		{
			displayName: 'Script',
			icon: Code2,
			href: `${base}/scripts/add`
		},
		...(HOME_SHOW_CREATE_FLOW
			? [{ displayName: 'Flow', icon: BarsStaggered, href: `${base}/flows/add` } as Item]
			: []),
		...(HOME_SHOW_CREATE_APP
			? [
					{
						displayName: 'App',
						icon: LayoutDashboard,
						submenuItems: [
							{ displayName: 'Low-code app', href: `${base}/apps/add` },
							{ displayName: 'Full-code app', href: `${base}/apps_raw/add` }
						]
					} as Item
				]
			: []),
		{
			displayName: 'Workflow-as-Code',
			icon: Code2,
			submenuItems: [
				{ displayName: 'TypeScript', href: `${base}/scripts/add?wac=typescript` },
				{ displayName: 'Python', href: `${base}/scripts/add?wac=python` }
			]
		}
	])

	let importItems: Item[] = $derived([
		{
			displayName: 'Import script',
			icon: Code2,
			action: () => wacDrawer?.openDrawer?.()
		},
		...(HOME_SHOW_CREATE_FLOW
			? [
					{
						displayName: 'Import flow',
						icon: BarsStaggered,
						action: () => flowDrawer?.openDrawer?.()
					} as Item
				]
			: []),
		...(HOME_SHOW_CREATE_APP
			? [
					{
						displayName: 'Import low-code app',
						icon: LayoutDashboard,
						action: () => {
							appKind = 'lowcode'
							appImportType = 'yaml'
							appDrawer?.openDrawer?.()
						}
					} as Item,
					{
						displayName: 'Import full-code app',
						icon: LayoutDashboard,
						action: () => {
							appKind = 'fullcode'
							appImportType = 'yaml'
							appDrawer?.openDrawer?.()
						}
					} as Item
				]
			: [])
	])
</script>

<div class="flex flex-row gap-2 items-center">
	<DropdownV2
		items={newItems}
		placement="bottom-end"
		aiId="home-create-new"
		aiDescription="Create a new script, flow, app or workflow-as-code"
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Plus }}
				endIcon={{ icon: ChevronDown }}
			>
				New
			</Button>
		{/snippet}
	</DropdownV2>

	<DropdownV2
		items={importItems}
		placement="bottom-end"
		aiId="home-import"
		aiDescription="Import a script, flow or app from YAML/JSON"
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				variant="default"
				unifiedSize="md"
				startIcon={{ icon: Download }}
				endIcon={{ icon: ChevronDown }}
			>
				Import
			</Button>
		{/snippet}
	</DropdownV2>
</div>

<!-- Import flow drawer -->
<Drawer bind:this={flowDrawer} size="800px">
	<DrawerContent title="Import flow from YAML/JSON" on:close={() => flowDrawer?.closeDrawer?.()}>
		<Tabs bind:selected={flowImportType}>
			<Tab value="yaml" label="YAML" />
			<Tab value="json" label="JSON" />
			{#snippet content()}
				<div class="relative pt-2 h-full">
					{#key flowImportType}
						{#await import('$lib/components/SimpleEditor.svelte')}
							<Loader2 class="animate-spin" />
						{:then Module}
							<Module.default
								bind:code={pendingFlowRaw}
								lang={flowImportType}
								class="h-full"
								fixedOverflowWidgets={false}
							/>
						{/await}
					{/key}
				</div>
			{/snippet}
		</Tabs>
		{#snippet actions()}
			<Button size="sm" on:click={importFlowRaw}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<!-- Import workflow-as-code drawer -->
<Drawer bind:this={wacDrawer} size="800px">
	<DrawerContent
		title="Import script / Workflow-as-Code"
		on:close={() => wacDrawer?.closeDrawer?.()}
	>
		<Tabs bind:selected={wacImportType}>
			<Tab value="yaml" label="YAML" />
			<Tab value="json" label="JSON" />
			{#snippet content()}
				<div class="relative pt-2 h-full">
					{#key wacImportType}
						{#await import('$lib/components/SimpleEditor.svelte')}
							<Loader2 class="animate-spin" />
						{:then Module}
							<Module.default
								bind:code={pendingWacRaw}
								lang={wacImportType}
								class="h-full"
								fixedOverflowWidgets={false}
							/>
						{/await}
					{/key}
				</div>
			{/snippet}
		</Tabs>
		{#snippet actions()}
			<Button size="sm" on:click={importWacRaw}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<!-- Import app drawer -->
<Drawer bind:this={appDrawer} size="800px">
	<DrawerContent
		title={appKind === 'fullcode' ? 'Import full-code app' : 'Import low-code app'}
		on:close={() => appDrawer?.closeDrawer?.()}
	>
		<Tabs bind:selected={appImportType}>
			<Tab value="yaml" label="YAML" />
			<Tab value="json" label="JSON" />
			{#snippet content()}
				<div class="relative pt-2 h-full">
					{#key appImportType}
						{#await import('$lib/components/SimpleEditor.svelte')}
							<Loader2 class="animate-spin" />
						{:then Module}
							<Module.default
								bind:code={pendingAppRaw}
								lang={appImportType}
								class="h-full"
								fixedOverflowWidgets={false}
							/>
						{/await}
					{/key}
				</div>
			{/snippet}
		</Tabs>
		{#snippet actions()}
			<Button size="sm" on:click={importAppRaw}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
