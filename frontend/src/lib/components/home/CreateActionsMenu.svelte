<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import {
		Plus,
		Code2,
		LayoutDashboard,
		ChevronRight,
		Loader2,
		Workflow,
		Import
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { PythonIcon, TypeScriptIcon } from '$lib/components/common/languageIcons'
	import { HOME_SHOW_CREATE_FLOW, HOME_SHOW_CREATE_APP } from '$lib/consts'
	import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
	import { importStore } from '$lib/components/apps/store'
	import YAML from 'yaml'

	type Variant = {
		label: string
		icon: typeof PythonIcon
		onSelect: () => void
	}

	/** an importable artifact kind handled by the shared YAML/JSON import drawer */
	type ImportKind = 'flow' | 'wac' | 'app-lowcode' | 'app-fullcode'

	type Extra = {
		label: string
		onSelect: () => void
	}

	type Option = {
		key: string
		label: string
		icon: typeof Code2 | typeof BarsStaggered
		/** tailwind accent token used for the icon tile / hover border */
		accent: string
		tagline: string
		description: string
		bullets: string[]
		onSelect: () => void
		/** optional sub-choices shown in place of the single create button */
		variants?: Variant[]
		/** optional pill shown next to the label; class is the full static tailwind tone */
		badge?: { label: string; class: string }
		/** secondary actions (import from YAML/JSON, pipeline, …) shown in the detail panel */
		extras?: Extra[]
	}

	// kept static so tailwind doesn't purge the badge tones
	const badgeAdvanced = 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300'
	const badgeLegacy = 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300'
	const badgeAlpha = 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300'

	const allOptions: Option[] = [
		{
			key: 'script',
			label: 'Script',
			icon: Code2,
			accent: 'blue',
			tagline: 'A single standalone script',
			description:
				'Author a script in Python, TypeScript, Go, Bash, SQL, Rust, PHP and more. Windmill auto-generates an input UI, deploys it instantly and exposes it as an API.',
			bullets: [
				'20+ languages',
				'Auto-generated UI from parameters',
				'Instant deploy & versioning'
			],
			onSelect: () => goto(`${base}/scripts/add`)
		},
		...(HOME_SHOW_CREATE_FLOW
			? ([
					{
						key: 'flow',
						label: 'Flow',
						icon: BarsStaggered,
						accent: 'teal',
						tagline: 'Compose scripts into a workflow',
						description:
							'Visual builder for chaining scripts together with branches, loops, error handlers, approvals and retries. Each step can be reused from the workspace or the Hub.',
						bullets: [
							'Drag-and-drop steps',
							'Branches, loops & error handling',
							'Suspend / approval steps'
						],
						onSelect: () => goto(`${base}/flows/add`),
						extras: [{ label: 'Import flow', onSelect: () => openImport('flow') }]
					}
				] as Option[])
			: []),
		...(HOME_SHOW_CREATE_APP
			? ([
					{
						key: 'app-fullcode',
						label: 'App (full-code)',
						icon: LayoutDashboard,
						accent: 'purple',
						tagline: 'Build with React or Svelte',
						description:
							'Full control over the UI with React or Svelte and a powerful AI agent. Best for complex apps that need full flexibility.',
						bullets: ['React or Svelte', 'Full flexibility & control', 'AI-assisted authoring'],
						onSelect: () => goto(`${base}/apps_raw/add`),
						extras: [{ label: 'Import full-code app', onSelect: () => openImport('app-fullcode') }]
					}
				] as Option[])
			: []),
		...(HOME_SHOW_CREATE_FLOW
			? ([
					{
						key: 'wac',
						label: 'Workflow-as-Code',
						icon: Code2,
						accent: 'purple',
						tagline: 'Express a workflow purely in code',
						description:
							'Write the whole workflow as a single Python or TypeScript script using the Windmill SDK — parallelism, branching and step orchestration expressed as plain code.',
						bullets: [
							'Python or TypeScript',
							'Full control via the SDK',
							'Versioned as a regular script'
						],
						onSelect: () => goto(`${base}/scripts/add?wac=typescript`),
						badge: { label: 'Advanced', class: badgeAdvanced },
						variants: [
							{
								label: 'TypeScript',
								icon: TypeScriptIcon,
								onSelect: () => goto(`${base}/scripts/add?wac=typescript`)
							},
							{
								label: 'Python',
								icon: PythonIcon,
								onSelect: () => goto(`${base}/scripts/add?wac=python`)
							}
						],
						extras: [{ label: 'Import Workflow-as-Code', onSelect: () => openImport('wac') }]
					},
					{
						key: 'pipeline',
						label: 'Data pipelines',
						icon: Workflow,
						accent: 'indigo',
						tagline: 'Compose data ingestion & transforms',
						description:
							'Visual editor for data pipelines — chain ingestion, transformation and materialization steps with partitions and incremental processing.',
						bullets: [
							'Ingest, transform & materialize',
							'Partitioned & incremental',
							'Asset-aware lineage'
						],
						onSelect: () => goto(`${base}/pipeline`),
						badge: { label: 'Alpha', class: badgeAlpha }
					}
				] as Option[])
			: []),
		...(HOME_SHOW_CREATE_APP
			? ([
					{
						key: 'app-lowcode',
						label: 'App (low-code)',
						icon: LayoutDashboard,
						accent: 'orange',
						tagline: 'Drag-and-drop UI builder',
						description:
							'Assemble an internal UI from 60+ components wired to your scripts and flows. Best for simple apps or apps that need minimal customization.',
						bullets: ['60+ ready-made components', 'No code required', 'Backed by scripts & flows'],
						onSelect: () => goto(`${base}/apps/add`),
						badge: { label: 'Legacy', class: badgeLegacy },
						extras: [{ label: 'Import low-code app', onSelect: () => openImport('app-lowcode') }]
					}
				] as Option[])
			: [])
	]

	// tailwind needs the full class names statically — map accent token -> classes
	const accentClasses: Record<
		string,
		{ tile: string; iconText: string; activeBg: string; activeBorder: string }
	> = {
		blue: {
			tile: 'bg-blue-100 dark:bg-blue-900/40',
			iconText: 'text-blue-600 dark:text-blue-400',
			activeBg: 'bg-blue-50 dark:bg-blue-900/20',
			activeBorder: 'border-blue-500 dark:border-blue-400'
		},
		teal: {
			tile: 'bg-teal-100 dark:bg-teal-900/40',
			iconText: 'text-teal-600 dark:text-teal-400',
			activeBg: 'bg-teal-50 dark:bg-teal-900/20',
			activeBorder: 'border-teal-500 dark:border-teal-400'
		},
		purple: {
			tile: 'bg-purple-100 dark:bg-purple-900/40',
			iconText: 'text-purple-600 dark:text-purple-400',
			activeBg: 'bg-purple-50 dark:bg-purple-900/20',
			activeBorder: 'border-purple-500 dark:border-purple-400'
		},
		orange: {
			tile: 'bg-orange-100 dark:bg-orange-900/40',
			iconText: 'text-orange-600 dark:text-orange-400',
			activeBg: 'bg-orange-50 dark:bg-orange-900/20',
			activeBorder: 'border-orange-500 dark:border-orange-400'
		},
		indigo: {
			tile: 'bg-indigo-100 dark:bg-indigo-900/40',
			iconText: 'text-indigo-600 dark:text-indigo-400',
			activeBg: 'bg-indigo-50 dark:bg-indigo-900/20',
			activeBorder: 'border-indigo-500 dark:border-indigo-400'
		}
	}

	let open = $state(false)
	let activeKey = $state(allOptions[0]?.key)
	let active = $derived(allOptions.find((o) => o.key === activeKey) ?? allOptions[0])
	let activeAc = $derived(accentClasses[active.accent])

	let closeTimeout: ReturnType<typeof setTimeout> | undefined
	function scheduleOpen() {
		if (closeTimeout) clearTimeout(closeTimeout)
		open = true
	}
	function scheduleClose() {
		if (closeTimeout) clearTimeout(closeTimeout)
		closeTimeout = setTimeout(() => (open = false), 120)
	}

	// shared YAML/JSON import drawer, reused by every "Import …" extra
	let importDrawer: Drawer | undefined = $state(undefined)
	let importKind: ImportKind = $state('flow')
	let importType: 'yaml' | 'json' = $state('yaml')
	let importRaw: string = $state('')

	const importTitles: Record<ImportKind, string> = {
		flow: 'Import flow',
		wac: 'Import Workflow-as-Code',
		'app-lowcode': 'Import low-code app',
		'app-fullcode': 'Import full-code app'
	}

	function openImport(kind: ImportKind) {
		importKind = kind
		importType = 'yaml'
		importRaw = ''
		open = false
		importDrawer?.openDrawer?.()
	}

	async function runImport() {
		const parsed = importType === 'yaml' ? YAML.parse(importRaw) : JSON.parse(importRaw)
		if (importKind === 'flow') {
			importFlowStore.set(parsed)
			await goto(`${base}/flows/add`)
		} else if (importKind === 'wac') {
			importScriptStore.set(parsed)
			await goto(`${base}/scripts/add?import=true`)
		} else if (importKind === 'app-fullcode') {
			// /apps_raw/add does a full reload (cross-origin isolation), so the in-memory
			// store would be lost — hand the payload over via sessionStorage instead.
			sessionStorage.setItem('rawAppImport', JSON.stringify(parsed))
			await goto(`${base}/apps_raw/add`)
		} else {
			importStore.set(parsed)
			await goto(`${base}/apps/add`)
		}
		importDrawer?.closeDrawer?.()
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="relative"
	onmouseenter={scheduleOpen}
	onmouseleave={scheduleClose}
	onfocusin={scheduleOpen}
	onfocusout={scheduleClose}
>
	<Button
		id="create-new-button"
		aiId="home-create-new"
		aiDescription="Create a new script, flow or app"
		unifiedSize="md"
		variant="accent"
		startIcon={{ icon: Plus }}
		onClick={() => active?.onSelect()}
	>
		New
	</Button>

	{#if open && active}
		<div class="absolute right-0 top-full z-50 pt-2" role="menu" tabindex="-1">
			<div
				class="flex flex-row rounded-lg border border-gray-200 dark:border-gray-700 bg-surface shadow-xl overflow-hidden"
				style="width: 780px;"
			>
				<!-- explanation of the highlighted editor -->
				<div class="flex flex-col gap-3 p-5 flex-1 min-w-0">
					<div class="flex flex-row items-center gap-3">
						<div
							class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 {activeAc.tile}"
						>
							<active.icon size={26} class={activeAc.iconText} />
						</div>
						<div class="min-w-0">
							<div class="flex flex-row items-center gap-2">
								<h3 class="font-semibold text-primary leading-tight">{active.label}</h3>
								{#if active.badge}
									<span
										class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide {active
											.badge.class}"
									>
										{active.badge.label}
									</span>
								{/if}
							</div>
							<p class="text-xs text-tertiary">{active.tagline}</p>
						</div>
					</div>

					<p class="text-sm text-secondary leading-relaxed">{active.description}</p>

					<ul class="flex flex-col gap-1.5 mt-1">
						{#each active.bullets as bullet (bullet)}
							<li class="flex flex-row items-center gap-2 text-xs text-secondary">
								<ChevronRight size={14} class={activeAc.iconText} />
								{bullet}
							</li>
						{/each}
					</ul>

					{#if active.variants || active.extras}
						<div class="mt-auto flex flex-col gap-2 pt-2">
							{#if active.variants}
								<div class="flex flex-row flex-wrap gap-2">
									{#each active.variants as variant (variant.label)}
										{@const VariantIcon = variant.icon}
										<Button unifiedSize="sm" variant="accent" onClick={() => variant.onSelect()}>
											<VariantIcon width={16} height={16} />
											{variant.label}
										</Button>
									{/each}
								</div>
							{/if}
							{#if active.extras}
								<div class="flex flex-row flex-wrap gap-2">
									{#each active.extras as extra (extra.label)}
										<Button
											unifiedSize="sm"
											variant="default"
											startIcon={{ icon: Import }}
											onClick={() => extra.onSelect()}
										>
											{extra.label}
										</Button>
									{/each}
								</div>
							{/if}
						</div>
					{/if}
				</div>

				<!-- option list -->
				<div class="flex flex-col gap-0.5 p-2 w-[21rem] shrink-0">
					{#each allOptions as option, i (option.key)}
						{@const ac = accentClasses[option.accent]}
						{@const isActive = option.key === activeKey}
						{#if option.badge && !allOptions[i - 1]?.badge}
							<div class="mx-1 mt-3 mb-3 border-t border-gray-200 dark:border-gray-700"></div>
						{/if}
						<button
							class="flex flex-row items-center gap-3 rounded-md border px-2 py-2 text-left cursor-pointer transition-colors {isActive
								? `${ac.activeBg} ${ac.activeBorder}`
								: 'border-transparent hover:bg-surface-hover'}"
							onmouseenter={() => (activeKey = option.key)}
							onfocus={() => (activeKey = option.key)}
							onclick={() => option.onSelect()}
							role="menuitem"
						>
							<div class="w-8 h-8 rounded-md flex items-center justify-center shrink-0 {ac.tile}">
								<option.icon size={18} class={ac.iconText} />
							</div>
							<span class="text-sm font-medium text-primary flex-1 min-w-0 whitespace-nowrap">
								{option.label}
							</span>
							{#if option.badge}
								<span
									class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide {option
										.badge.class}"
								>
									{option.badge.label}
								</span>
							{/if}
							<ChevronRight
								size={16}
								class="transition-opacity {isActive
									? `opacity-100 ${ac.iconText}`
									: 'opacity-0 text-tertiary'}"
							/>
						</button>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- shared import drawer (YAML / JSON) for the detail-panel "Import …" actions -->
<Drawer bind:this={importDrawer} size="800px">
	<DrawerContent title={importTitles[importKind]} on:close={() => importDrawer?.closeDrawer?.()}>
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
								bind:code={importRaw}
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
			<Button size="sm" onClick={runImport}>Import</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
