<script lang="ts">
	// Gallery of every icon component, at /kitchen_sink/icons.
	//
	// Sourced by glob rather than from APP_TO_ICON_COMPONENT, because the icons the
	// map does NOT reach are exactly what this page needs to surface.
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import tokensJson from '$lib/assets/tokens/tokens.json'
	import { darkModeName, lightModeName } from '$lib/assets/tokens/colorTokensConfig'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'

	// Tailwind's `dark:` variants work off a descendant selector, but the color tokens
	// behind text-secondary/bg-surface are declared on `html.dark` (tailwind.config.cjs
	// addBase), which a wrapper div cannot reach. Redeclaring them inline makes each
	// half of a tile a real light/dark context whatever theme the app itself is in.
	function hexToRgb(hex: string): string {
		if (!hex.startsWith('#')) return hex
		const h = hex.slice(1)
		const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.substring(i, i + 2), 16))
		if (h.length === 8) {
			return `${r} ${g} ${b} / ${(parseInt(h.substring(6, 8), 16) / 255).toFixed(3)}`
		}
		return `${r} ${g} ${b}`
	}

	function themeVars(mode: string): string {
		const mapping = (tokensJson.tokens as Record<string, Record<string, string>>)[mode]
		return Object.entries(mapping)
			.map(([key, value]) => `--color-${key}:${hexToRgb(value)}`)
			.join(';')
	}

	const lightVars = themeVars(lightModeName)
	const darkVars = themeVars(darkModeName)

	// Includes triggers/ so the monochrome trigger variants sit next to the full-colour
	// marks they shadow — the pair is the thing you need to see when deciding which to use.
	const modules = import.meta.glob('../../../lib/components/icons/**/*.svelte', { eager: true })

	const entries = Object.entries(modules)
		.map(([path, mod]) => {
			const file = path.split('/').pop()?.replace('.svelte', '') ?? path
			const folder = path.split('/').slice(-2, -1)[0]
			return {
				name: folder === 'icons' ? file : `${folder}/${file}`,
				component: (mod as { default: any }).default
			}
		})
		.sort((a, b) => a.name.localeCompare(b.name))

	// Reverse the map by component identity: component .name is unreliable once minified.
	const nameByComponent = new Map(entries.map((e) => [e.component, e.name]))
	const keysByName = $derived.by(() => {
		const out: Record<string, string[]> = {}
		for (const [key, component] of Object.entries(APP_TO_ICON_COMPONENT)) {
			const name = nameByComponent.get(component)
			if (name) (out[name] ??= []).push(key)
		}
		return out
	})

	let search = $state('')
	let size = $state(24)
	let colorOverride = $state('')
	let classOverride = $state('')
	let onlyMapped = $state(false)
	let showDark = $state(true)
	let showBox = $state(false)

	// The theme's color variables live on `html.dark`, so only that class puts the page
	// chrome itself in dark mode. Seeded from the app's current theme so mounting the
	// page never flips it, and restored on leaving.
	let pageDark = $state(
		typeof document !== 'undefined' && document.documentElement.classList.contains('dark')
	)

	$effect(() => {
		const el = document.documentElement
		const previous = el.classList.contains('dark')
		el.classList.toggle('dark', pageDark)
		return () => el.classList.toggle('dark', previous)
	})

	let iconProps = $derived({
		size,
		// Numbers, not '24px' strings: the library has both `width?: string` components
		// that emit {width} verbatim and `width?: number` ones that emit `${width}px`.
		// A number is a valid SVG length under both; a px string yields "24pxpx" on the
		// second kind, which is invalid and falls back to the 300x150 default size.
		width: size,
		height: size,
		// Only forwarded when set: most components declare neither prop, and passing
		// them unconditionally logs an unknown-prop warning for every icon on screen.
		...(colorOverride ? { color: colorOverride } : {}),
		...(classOverride ? { class: classOverride } : {})
	})

	let shown = $derived(
		entries.filter((e) => {
			const keys = keysByName[e.name] ?? []
			if (onlyMapped && keys.length === 0) return false
			if (!search) return true
			const q = search.toLowerCase()
			return e.name.toLowerCase().includes(q) || keys.some((k) => k.includes(q))
		})
	)

	const sizeItems = [16, 20, 24, 32, 48, 64].map((v) => ({ label: `${v}px`, value: v }))
</script>

<!-- Outlines the icon's nominal size box, so the gap between the artwork and the box it is
	 given can be read off the page. Lucide leaves ~4% of the box as margin on its tightest side. -->
{#snippet mark(Icon: any)}
	{#if showBox}
		<div
			class="flex items-center justify-center outline outline-1 outline-border-accent"
			style="width: {size}px; height: {size}px"
		>
			<Icon {...iconProps} />
		</div>
	{:else}
		<Icon {...iconProps} />
	{/if}
{/snippet}

<div class="h-full overflow-auto bg-surface">
	<div class="max-w-[1400px] mx-auto flex flex-col gap-4 p-6">
		<div class="flex flex-col gap-1">
			<h1 class="text-lg font-semibold text-emphasis">Icon library</h1>
			<p class="text-sm text-secondary">
				Every component in <code class="text-xs">lib/components/icons</code>. Each tile renders the
				icon on Windmill's light surface and, when enabled, its dark surface — so a monochrome
				icon's light/dark pair can be checked side by side without switching the app theme. An icon
				only differs between the two halves if it inherits <code class="text-xs">currentColor</code>
				or carries a <code class="text-xs">dark:</code> class; hardcoded fills look the same on both.
			</p>
		</div>

		<div class="flex flex-wrap gap-4 items-end border rounded-md p-3 bg-surface-secondary">
			<div class="w-64">
				<Label label="Search">
					<TextInput bind:value={search} inputProps={{ placeholder: 'name or resource type' }} />
				</Label>
			</div>
			<div class="w-28">
				<Label label="Size">
					<Select items={sizeItems} bind:value={size} />
				</Label>
			</div>
			<div class="w-40">
				<Label label="color prop">
					<TextInput bind:value={colorOverride} inputProps={{ placeholder: '#ff0000' }} />
				</Label>
			</div>
			<div class="w-52">
				<Label label="class prop">
					<TextInput bind:value={classOverride} inputProps={{ placeholder: 'text-teal-500' }} />
				</Label>
			</div>
			<Toggle bind:checked={showDark} options={{ right: 'Dark surface' }} />
			<Toggle bind:checked={showBox} options={{ right: 'Icon box' }} />
			<Toggle bind:checked={onlyMapped} options={{ right: 'Mapped to a resource type' }} />
			<Toggle bind:checked={pageDark} options={{ right: 'Dark page' }} />
		</div>

		<div class="text-xs text-hint">
			{shown.length} of {entries.length} components
			{#if colorOverride || classOverride}
				· overrides apply only to components declaring that prop
			{/if}
			{#if pageDark && showDark}
				· with the page dark, <code>dark:</code> classes also match inside the light half — read the
				pairs with it off
			{/if}
		</div>

		<div class="grid gap-2 grid-cols-[repeat(auto-fill,minmax(160px,1fr))]">
			{#each shown as entry (entry.name)}
				{@const Icon = entry.component}
				{@const keys = keysByName[entry.name] ?? []}
				<div class="border rounded-md overflow-hidden flex flex-col">
					<div class="flex">
						<div
							class="flex-1 flex items-center justify-center py-5 bg-surface text-secondary min-h-[72px]"
							style={lightVars}
						>
							{@render mark(Icon)}
						</div>
						{#if showDark}
							<div
								class="dark flex-1 flex items-center justify-center py-5 bg-surface text-secondary min-h-[72px]"
								style={darkVars}
							>
								{@render mark(Icon)}
							</div>
						{/if}
					</div>
					<div class="px-2 py-1.5 border-t bg-surface flex flex-col gap-0.5">
						<div class="text-2xs font-mono text-primary truncate" title={entry.name}>
							{entry.name}
						</div>
						<div class="text-2xs text-hint truncate" title={keys.join(', ')}>
							{keys.length ? keys.join(', ') : 'unmapped'}
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>
