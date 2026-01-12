<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { LayoutDashboard, Loader2, Plus, Code2, FlaskConical } from 'lucide-svelte'
	import { importStore } from '../apps/store'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	import YAML from 'yaml'

	let drawer: Drawer | undefined = undefined
	let pendingRaw: string = $state('')

	let importType: 'yaml' | 'json' = $state('yaml')

	// Modal states
	let appTypeModalOpen = $state(false)
	let featurePreviewModalOpen = $state(false)

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
		featurePreviewModalOpen = true
	}

	function confirmFullCode() {
		featurePreviewModalOpen = false
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
					</p>
				</div>
				<br /><br />
				<div class="flex flex-row items-center gap-2">
					<Badge color="yellow">
						<FlaskConical size={12} class="mr-1" />
						Preview (unstable)
					</Badge>
				</div>
			</button>
		</div>
	</div>
</Modal>

<!-- Feature Preview Warning Modal -->
<Modal kind="X" bind:open={featurePreviewModalOpen} title="Feature Preview Notice">
	<div class="flex flex-col gap-4">
		<div
			class="flex items-start gap-3 p-4 rounded-lg bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800"
		>
			<FlaskConical size={24} class="text-yellow-600 dark:text-yellow-400 shrink-0 mt-0.5" />
			<div class="text-sm text-yellow-800 dark:text-yellow-200">
				<p class="font-semibold mb-2">Full-code Apps are in Feature Preview</p>
				<p>
					This feature is not yet stable and may change significantly before it reaches General
					Availability (GA), which is expected in <strong>January 2026</strong>.
				</p>
				<ul class="list-disc list-inside mt-2 space-y-1 text-yellow-700 dark:text-yellow-300">
					<li>APIs and interfaces may change without notice</li>
					<li>Some functionality may be incomplete or buggy</li>
					<li>Not recommended for production use yet</li>
				</ul>
			</div>
		</div>

		<div class="flex justify-end gap-3 pt-2">
			<Button
				color="light"
				size="sm"
				on:click={() => {
					featurePreviewModalOpen = false
					appTypeModalOpen = true
				}}
			>
				Go back
			</Button>
			<Button color="blue" size="sm" variant="accent" on:click={confirmFullCode}
				>I understand, continue</Button
			>
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
