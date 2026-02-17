<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { LayoutDashboard, Loader2, Plus, Code2 } from 'lucide-svelte'
	import { importStore } from '../apps/store'
	import YAML from 'yaml'

	let drawer: Drawer | undefined = undefined
	let pendingRaw: string = $state('')

	let importType: 'yaml' | 'json' = $state('yaml')

	let appTypeModalOpen = $state(false)

	async function importRaw() {
		$importStore = importType === 'yaml' ? YAML.parse(pendingRaw) : JSON.parse(pendingRaw)
		await goto('/apps/add?nodraft=true')
		drawer?.closeDrawer?.()
	}

	function openAppTypeModal() {
		appTypeModalOpen = true
	}

	function selectLowCode() {
		appTypeModalOpen = false
		goto(`${base}/apps/add?nodraft=true`)
	}

	function selectFullCode() {
		appTypeModalOpen = false
		goto(`${base}/apps_raw/add?nodraft=true`)
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
		<Button
			id="create-app-button"
			aiId="apps-create-actions-app"
			aiDescription="Create a new app"
			unifiedSize="lg"
			startIcon={{ icon: Plus }}
			endIcon={{ icon: LayoutDashboard }}
			on:click={openAppTypeModal}
			variant="accent"
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
			]}
		>
			<div class="flex flex-row items-center"> App </div>
		</Button>
	</div>

<!-- App Type Selection Modal -->
<Modal bind:open={appTypeModalOpen} title="Choose your app builder">
	<div class="flex flex-col gap-4 pr-4">
		<div class="grid grid-cols-2 gap-8">
			<!-- Low-code option -->
			<button
				class="flex flex-col items-center gap-3 p-6 rounded-lg border-2 border-gray-200 dark:border-gray-700 hover:border-orange-500 dark:hover:border-orange-400 hover:bg-orange-50 dark:hover:bg-orange-900/20 transition-all cursor-pointer group"
				onclick={selectLowCode}
			>
				<div
					class="w-32 h-32 rounded-xl bg-orange-100 dark:bg-orange-900/40 flex items-center justify-center group-hover:bg-orange-200 dark:group-hover:bg-orange-800/50 transition-colors"
				>
					<LayoutDashboard size={32} class="text-orange-600 dark:text-orange-400" />
				</div>
				<div class="text-center">
					<h3 class="font-semibold text-primary">Low-code App</h3>
					<p class="text-xs text-tertiary mt-1">
						Drag-and-drop UI builder with 60+ powerful components.

						<br /><br />
						Better for simple apps or apps that require minimal customization.
					</p>
				</div>
			</button>

			<button
				class="relative flex flex-col items-center gap-3 p-6 rounded-lg border-2 border-gray-200 dark:border-gray-700 hover:border-purple-500 dark:hover:border-purple-400 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-all cursor-pointer group"
				onclick={selectFullCode}
			>
				<div
					class="w-32 h-32 rounded-xl bg-purple-100 dark:bg-purple-900/40 flex items-center justify-center group-hover:bg-purple-200 dark:group-hover:bg-purple-800/50 transition-colors"
				>
					<Code2 size={32} class="text-purple-600 dark:text-purple-400" />
				</div>
				<div class="text-center">
					<h3 class="font-semibold text-primary">Full-code App</h3>
					<p class="text-xs text-tertiary mt-1">
						Build with React or Svelte with full control and a powerful AI agent.

					<br /><br />
					Better for complex apps or apps that require full flexibility and control.
					</p>
				</div>
			</button>
		</div>
	</div>
</Modal>

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
