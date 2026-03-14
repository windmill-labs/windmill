<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { PythonIcon, TypeScriptIcon } from '$lib/components/common/languageIcons'
	import { Code2, Loader2, Plus } from 'lucide-svelte'
	import YAML from 'yaml'

	const SKIP_FLOW_MODAL_KEY = 'windmill_skip_flow_modal'

	let drawer: Drawer | undefined = $state(undefined)
	let pendingRaw: string | undefined = $state(undefined)
	let importType: 'yaml' | 'json' = $state('yaml')
	let flowModalOpen = $state(false)
	let wacHovered = $state(false)
	let skipModal = $state(
		typeof localStorage !== 'undefined' && localStorage.getItem(SKIP_FLOW_MODAL_KEY) === 'true'
	)

	async function importRaw() {
		$importFlowStore =
			importType === 'yaml' ? YAML.parse(pendingRaw ?? '') : JSON.parse(pendingRaw ?? '')
		await goto('/flows/add')
		drawer?.closeDrawer?.()
	}

	function handleFlowClick() {
		if (skipModal) {
			goto(`${base}/flows/add?nodraft=true`)
		} else {
			flowModalOpen = true
		}
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

	function toggleSkipModal() {
		skipModal = !skipModal
		localStorage.setItem(SKIP_FLOW_MODAL_KEY, String(skipModal))
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
				label: 'Workflow-as-Code in Python',
				onClick: () => selectWacPython()
			},
			{
				label: 'Workflow-as-Code in TypeScript',
				onClick: () => selectWacTypescript()
			}
		]}
	>
		Flow
	</Button>
</div>

<!-- Flow Type Selection Modal -->
<Modal bind:open={flowModalOpen} title="Create a new flow">
	<div class="flex flex-col gap-4 pr-4">
		<div class="grid grid-cols-2 gap-8">
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
						Visual builder for composing scripts into workflows with branching, loops,
						and error handling.
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
				<!-- Alpha badge -->
				<div class="absolute top-2 right-2 z-10 px-2 py-0.5 rounded-full bg-purple-500 text-white text-2xs font-bold uppercase tracking-wide">
					Alpha
				</div>

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
							Write workflows as Python or TypeScript code
							as a regular Windmill script.
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
		</div>

		<div class="flex items-center gap-2 px-1">
			<Toggle size="xs" checked={skipModal} on:change={toggleSkipModal} />
			<span class="text-2xs text-tertiary">Always use the Flow editor (skip this modal)</span>
		</div>
	</div>
</Modal>

<!-- Import Drawer -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title="Import flow from YAML/JSON"
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
