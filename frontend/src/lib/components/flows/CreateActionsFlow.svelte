<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { PythonIcon, TypeScriptIcon } from '$lib/components/common/languageIcons'
	import { Code2, Loader2, NetworkIcon, Plus } from 'lucide-svelte'
	import YAML from 'yaml'

	let drawer: Drawer | undefined = $state(undefined)
	let wacDrawer: Drawer | undefined = $state(undefined)
	let pendingRaw: string | undefined = $state(undefined)
	let pendingWacRaw: string | undefined = $state(undefined)
	let importType: 'yaml' | 'json' = $state('yaml')
	let wacImportType: 'yaml' | 'json' = $state('yaml')
	let flowModalOpen = $state(false)
	let wacHovered = $state(false)

	async function importRaw() {
		$importFlowStore =
			importType === 'yaml' ? YAML.parse(pendingRaw ?? '') : JSON.parse(pendingRaw ?? '')
		await goto('/flows/add?nodraft=true')
		drawer?.closeDrawer?.()
	}

	async function importWacRaw() {
		const parsed =
			wacImportType === 'yaml' ? YAML.parse(pendingWacRaw ?? '') : JSON.parse(pendingWacRaw ?? '')
		$importScriptStore = parsed
		await goto(`${base}/scripts/add?import=true&nodraft=true`)
		wacDrawer?.closeDrawer?.()
	}

	function handleFlowClick() {
		flowModalOpen = true
	}

	function selectFlowEditor() {
		flowModalOpen = false
		goto(`${base}/flows/add?nodraft=true`)
	}

	function selectWacPython() {
		flowModalOpen = false
		goto(`${base}/scripts/add?nodraft=true&wac=python`)
	}

	function selectWacTypescript() {
		flowModalOpen = false
		goto(`${base}/scripts/add?nodraft=true&wac=typescript`)
	}

	function selectPipeline() {
		flowModalOpen = false
		goto(`${base}/pipeline`)
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
		on:click={handleFlowClick}
		variant="accent"
		dropdownItems={[
			{
				label: 'Import Flow',
				onClick: () => {
					drawer?.toggleDrawer?.()
				}
			},
			{
				label: 'Workflow-as-Code in TypeScript',
				onClick: () => selectWacTypescript()
			},
			{
				label: 'Workflow-as-Code in Python',
				onClick: () => selectWacPython()
			},
			{
				label: 'Import Workflow-as-Code',
				onClick: () => {
					wacDrawer?.toggleDrawer?.()
				}
			}
		]}
	>
		Flow
	</Button>
</div>

<!-- Flow Type Selection Modal -->
<!-- max-w-4xl widens the modal to fit three tiles comfortably; default
     `max-w-lg` (~512px) was sized for two tiles and squeezed the grid.
     kind="X" replaces the bottom Cancel button with a top-right close (X)
     so the action area is purely the three tile buttons. -->
<Modal bind:open={flowModalOpen} title="Create a new flow" class="sm:max-w-4xl" kind="X">
	<div class="flex flex-col gap-4 pr-4">
		<div class="grid grid-cols-3 gap-6">
			<!-- Flow Editor option -->
			<button
				class="flex flex-col items-center gap-3 p-6 rounded-lg border-2 border-gray-200 dark:border-gray-700 hover:border-blue-500 dark:hover:border-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all cursor-pointer group"
				onclick={selectFlowEditor}
			>
				<div
					class="w-32 h-32 rounded-xl bg-blue-100 dark:bg-blue-900/40 flex items-center justify-center group-hover:bg-blue-200 dark:group-hover:bg-blue-800/50 transition-colors"
				>
					<BarsStaggered size={32} class="text-blue-600 dark:text-blue-400" style="" />
				</div>
				<div class="text-center">
					<h3 class="font-semibold text-primary">Flow Editor</h3>
					<p class="text-xs text-tertiary mt-1">
						Visual builder for composing scripts into workflows with branching, loops, and error
						handling.
					</p>
				</div>
			</button>

			<!-- Workflow-as-Code option -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="relative rounded-lg border-2 border-gray-200 dark:border-gray-700 hover:border-purple-500 dark:hover:border-purple-400 transition-all cursor-pointer overflow-hidden"
				onmouseenter={() => (wacHovered = true)}
				onmouseleave={() => (wacHovered = false)}
			>
				<!-- Default state -->
				<div
					class="flex flex-col items-center gap-3 p-6 absolute inset-0 transition-all duration-300"
					style="opacity: {wacHovered ? 0 : 1}; transform: scale({wacHovered ? 0.95 : 1})"
				>
					<div
						class="w-32 h-32 rounded-xl bg-purple-100 dark:bg-purple-900/40 flex items-center justify-center transition-colors"
					>
						<Code2 size={32} class="text-purple-600 dark:text-purple-400" />
					</div>
					<div class="text-center">
						<h3 class="font-semibold text-primary whitespace-nowrap">Workflow-as-Code</h3>
						<p class="text-xs text-tertiary mt-1">
							Write workflows as Python or TypeScript code as a regular Windmill script.
						</p>
					</div>
				</div>

				<!-- Hover state: split into TypeScript / Python -->
				<div
					class="flex flex-col h-full transition-all duration-300"
					style="opacity: {wacHovered ? 1 : 0}; transform: scale({wacHovered ? 1 : 0.95})"
				>
					<button
						class="flex-1 flex items-center justify-center gap-3 p-4 hover:bg-purple-100 dark:hover:bg-purple-900/40 transition-colors border-b border-gray-200 dark:border-gray-700"
						onclick={selectWacTypescript}
					>
						<div
							class="w-12 h-12 rounded-xl bg-purple-100 dark:bg-purple-900/40 flex items-center justify-center"
						>
							<TypeScriptIcon width={24} height={24} />
						</div>
						<span class="font-semibold text-sm text-primary">TypeScript</span>
					</button>
					<button
						class="flex-1 flex items-center justify-center gap-3 p-4 hover:bg-purple-100 dark:hover:bg-purple-900/40 transition-colors"
						onclick={selectWacPython}
					>
						<div
							class="w-12 h-12 rounded-xl bg-purple-100 dark:bg-purple-900/40 flex items-center justify-center"
						>
							<PythonIcon width={24} height={24} />
						</div>
						<span class="font-semibold text-sm text-primary">Python</span>
					</button>
				</div>
			</div>

			<!-- Pipeline option -->
			<button
				class="relative flex flex-col items-center gap-3 p-6 rounded-lg border-2 border-gray-200 dark:border-gray-700 hover:border-emerald-500 dark:hover:border-emerald-400 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 transition-all cursor-pointer group"
				onclick={selectPipeline}
			>
				<!-- Alpha badge — pipeline is the newest of the three; same
				     positioning the WAC tile used so it reads consistently. -->
				<div
					class="absolute top-2 right-2 z-10 px-2 py-0.5 rounded-full bg-emerald-500 text-white text-2xs font-bold uppercase tracking-wide"
				>
					Alpha
				</div>
				<div
					class="w-32 h-32 rounded-xl bg-emerald-100 dark:bg-emerald-900/40 flex items-center justify-center group-hover:bg-emerald-200 dark:group-hover:bg-emerald-800/50 transition-colors"
				>
					<NetworkIcon size={32} class="text-emerald-600 dark:text-emerald-400" />
				</div>
				<div class="text-center">
					<h3 class="font-semibold text-primary">Pipeline</h3>
					<p class="text-xs text-tertiary mt-1">
						Asset-driven DAG of scripts. Trigger on schedule, asset change, or external event.
					</p>
				</div>
			</button>
		</div>
	</div>
</Modal>

<!-- Import Drawer -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Import flow from YAML/JSON" on:close={() => drawer?.toggleDrawer?.()}>
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

<!-- Import WAC Drawer -->
<Drawer bind:this={wacDrawer} size="800px">
	<DrawerContent title="Import Workflow-as-Code" on:close={() => wacDrawer?.toggleDrawer?.()}>
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
