<script lang="ts">
	import { onMount } from 'svelte'
	import { SquareArrowOutUpRight, Sparkles, Undo2 } from 'lucide-svelte'
	import { replaceState } from '$app/navigation'
	import { Button, RadioCard } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		CONFIG_MESSAGE,
		DEFAULT_CONFIG,
		KEY_MESSAGE,
		PRESETS,
		VARIANTS,
		ZEN_PAGE,
		decodeConfig,
		encodeConfig,
		type Config
	} from './model'

	// Home's favorites & recent section in every shape worth comparing, on a fake Home with
	// made-up data, beside the controls that switch between them. The page itself is a frame
	// (`./frame`), so its breakpoints follow the width picked here.

	const WIDTHS = [
		{ value: 'full', label: 'Full' },
		{ value: '1280', label: '1280' },
		{ value: '1024', label: '1024' },
		{ value: '768', label: '768' },
		{ value: '390', label: '390' }
	]

	type Choice<K extends keyof Config> = { key: K; label: string; options: [Config[K], string][] }

	const LEVERS: Choice<
		'spacing' | 'icons' | 'menu' | 'time' | 'stars' | 'treeRows' | 'listFrame' | 'column'
	>[] = [
		{
			key: 'spacing',
			label: 'Space between sections',
			options: [
				['tight', 'Tight (today)'],
				['airy', 'Airy']
			]
		},
		{
			key: 'icons',
			label: 'Kind icons',
			options: [
				['colour', 'Colour'],
				['muted', 'Grey until hover']
			]
		},
		{
			key: 'menu',
			label: '⋮ menus, cards and rows',
			options: [
				['always', 'Always'],
				['hover', 'On hover']
			]
		},
		{
			key: 'time',
			label: 'Edited time',
			options: [
				['always', 'Always'],
				['hover', 'On hover']
			]
		},
		{
			key: 'stars',
			label: 'Stars on favorite shortcuts',
			options: [
				['always', 'Always'],
				['hover', 'On hover']
			]
		},
		{
			key: 'treeRows',
			label: 'Folder and user rows',
			options: [
				['two', 'Count below (today)'],
				['one', 'Count beside']
			]
		},
		{
			key: 'listFrame',
			label: 'Items list',
			options: [
				['card', 'In a card'],
				['flat', 'On the page']
			]
		},
		{
			key: 'column',
			label: 'Column width',
			options: [
				['wide', '1120px (today)'],
				['narrow', '960px']
			]
		}
	]

	const SWITCHES: { key: 'chips' | 'composer' | 'label' | 'dark'; label: string }[] = [
		{ key: 'chips', label: 'Owner chips above the tree' },
		{ key: 'composer', label: 'AI composer' },
		{ key: 'label', label: 'Section label' },
		{ key: 'dark', label: 'Dark mode' }
	]

	// The address bar, not `page.url`: the shallow `replaceState` below moves the first and leaves
	// the second at the last navigation, so a remount would read stale settings from it.
	const params = new URLSearchParams(location.search)
	const initial = decodeConfig(params)
	let config = $state<Config>(initial)
	let width = $state(params.get('width') ?? 'full')
	let frame: HTMLIFrameElement | undefined = $state(undefined)
	// Set once: later changes reach the frame as messages, so it never reloads.
	const frameSrc = `/kitchen_sink/home_cards/frame?${encodeConfig(initial)}`

	function post() {
		frame?.contentWindow?.postMessage(
			{ type: CONFIG_MESSAGE, config: $state.snapshot(config) },
			location.origin
		)
	}

	function syncUrl() {
		const next = encodeConfig(config)
		if (width !== 'full') next.set('width', width)
		const query = next.toString()
		replaceState(query ? `?${query}` : location.pathname, {})
	}

	function update(patch: Partial<Config>) {
		config = { ...config, ...patch }
		document.documentElement.classList.toggle('dark', config.dark)
		post()
		syncUrl()
	}

	function setWidth(next: string) {
		width = next
		syncUrl()
	}

	function cycle(step: number) {
		const i = VARIANTS.findIndex((v) => v.value === config.variant)
		update({ variant: VARIANTS[(i + step + VARIANTS.length) % VARIANTS.length].value })
	}

	function onArrow(key: string) {
		if (key === 'ArrowLeft') cycle(-1)
		if (key === 'ArrowRight') cycle(1)
	}

	function resetPage() {
		const { variant, preset, view, dark } = config
		update({ ...DEFAULT_CONFIG, variant, preset, view, dark })
	}

	onMount(() => {
		// Follow the app's theme unless the link says otherwise.
		if (!params.has('dark') && document.documentElement.classList.contains('dark')) {
			update({ dark: true })
		} else {
			document.documentElement.classList.toggle('dark', config.dark)
		}
		function onMessage(e: MessageEvent) {
			if (e.origin === location.origin && e.data?.type === KEY_MESSAGE) onArrow(e.data.key)
		}
		window.addEventListener('message', onMessage)
		return () => window.removeEventListener('message', onMessage)
	})
</script>

<svelte:head><title>Home cards</title></svelte:head>

<svelte:window
	onkeydown={(e) => {
		// A toggle group moves its own selection with the arrows.
		const t = e.target as HTMLElement | null
		if (t?.closest('input, textarea, [role="listbox"], [role="combobox"], [role="group"]')) return
		onArrow(e.key)
	}}
/>

<div class="flex h-screen bg-surface-secondary text-primary">
	<aside class="flex w-80 shrink-0 flex-col gap-6 overflow-y-auto border-r bg-surface p-4">
		<div class="flex flex-col gap-1">
			<h1 class="text-sm font-semibold text-emphasis">Home cards</h1>
			<p class="text-2xs text-secondary">
				A fake Home with made-up data. Stars and "Remove from recent" change only this page.
			</p>
		</div>

		<section class="flex flex-col gap-2">
			<div class="flex items-baseline justify-between">
				<span class="text-xs font-semibold text-emphasis">Variant</span>
				<span class="text-2xs text-hint">← → to switch</span>
			</div>
			<div class="flex flex-col gap-1.5" role="radiogroup">
				{#each VARIANTS as v (v.value)}
					<RadioCard
						label={v.label}
						description={v.description}
						selected={config.variant === v.value}
						onSelect={() => update({ variant: v.value })}
						showRadio={false}
						class="p-2.5"
					/>
				{/each}
			</div>
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">Workspace</span>
			<Select
				items={PRESETS}
				bind:value={() => config.preset, (v) => update({ preset: v ?? 'typical' })}
				size="sm"
			/>
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">View</span>
			<ToggleButtonGroup
				selected={config.view}
				onSelected={(v) => update({ view: v as Config['view'] })}
			>
				{#snippet children({ item })}
					<ToggleButton value="page" label="Whole page" {item} size="sm" />
					<ToggleButton value="sections" label="All variants stacked" {item} size="sm" />
				{/snippet}
			</ToggleButtonGroup>
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">Frame width</span>
			<ToggleButtonGroup selected={width} onSelected={setWidth}>
				{#snippet children({ item })}
					{#each WIDTHS as w (w.value)}
						<ToggleButton value={w.value} label={w.label} {item} size="sm" />
					{/each}
				{/snippet}
			</ToggleButtonGroup>
		</section>

		<section class="flex flex-col gap-3">
			<div class="flex items-center justify-between">
				<span class="text-xs font-semibold text-emphasis">Page settings</span>
				<div class="flex gap-1">
					<Button
						variant="subtle"
						unifiedSize="xs"
						startIcon={{ icon: Undo2 }}
						title="Back to today's spacing, icons and frames"
						onClick={resetPage}
					>
						Today
					</Button>
					<Button
						variant="default"
						unifiedSize="xs"
						startIcon={{ icon: Sparkles }}
						title="Airy spacing, grey icons, menus on hover, no owner chips"
						onClick={() => update(ZEN_PAGE)}
					>
						Calm
					</Button>
				</div>
			</div>
			{#each LEVERS as lever (lever.key)}
				<div class="flex flex-col gap-1">
					<span class="text-2xs text-secondary">{lever.label}</span>
					<ToggleButtonGroup
						selected={config[lever.key]}
						onSelected={(v) => update({ [lever.key]: v } as Partial<Config>)}
					>
						{#snippet children({ item })}
							{#each lever.options as [value, label] (value)}
								<ToggleButton {value} {label} {item} size="sm" />
							{/each}
						{/snippet}
					</ToggleButtonGroup>
				</div>
			{/each}
			<div class="flex flex-col gap-2 pt-1">
				{#each SWITCHES as s (s.key)}
					<Toggle
						size="sm"
						bind:checked={() => config[s.key], (v) => update({ [s.key]: v } as Partial<Config>)}
						options={{ right: s.label }}
					/>
				{/each}
			</div>
		</section>

		<div class="mt-auto">
			<Button
				unifiedSize="sm"
				variant="subtle"
				startIcon={{ icon: SquareArrowOutUpRight }}
				href={`/kitchen_sink/home_cards/frame?${encodeConfig(config)}`}
				target="_blank"
			>
				Open the page alone
			</Button>
		</div>
	</aside>

	<main class="flex min-w-0 flex-1 justify-center overflow-auto p-4">
		<!-- 2px over the width picked: the border sits inside it, and the page must get all of it
		     for the breakpoints to land where they do on a real screen. -->
		<div
			class="h-full shrink-0 overflow-hidden rounded-md border bg-surface"
			style:width={width === 'full' ? '100%' : `${Number(width) + 2}px`}
		>
			<iframe bind:this={frame} title="Home" src={frameSrc} class="h-full w-full" onload={post}
			></iframe>
		</div>
	</main>
</div>
