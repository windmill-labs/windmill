<script lang="ts">
	// The artifact document at /kitchen_sink/artifacts: the real renderer (`ArtifactBody`), HTML
	// export and PDF download, restyled live through `labCss`. Development builds only, because
	// trying fonts means loading them from Google Fonts, which a deployed instance must not do.
	import { PersistedState } from 'runed'
	import type { Attachment } from 'svelte/attachments'
	import { Copy, FileDown, FileText, RotateCcw } from 'lucide-svelte'
	import ArtifactBody from '$lib/components/copilot/chat/artifacts/ArtifactBody.svelte'
	import {
		artifactFontCss,
		artifactHtmlDocument
	} from '$lib/components/copilot/chat/artifacts/artifactExport'
	import { printArtifact } from '$lib/components/copilot/chat/artifacts/printArtifact'
	import { Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { copyToClipboard } from '$lib/utils'
	import LabSlider from './LabSlider.svelte'
	import {
		DEFAULT_SETTINGS,
		FONTS,
		INTER_STYLES,
		PRESETS,
		SETTINGS_KEY,
		fontLinks,
		labCss,
		type LabSettings
	} from './lab'
	import plan from './samples/plan.md?raw'
	import incident from './samples/incident.md?raw'
	import elements from './samples/elements.md?raw'

	const enabled = import.meta.env.DEV

	const SAMPLES = [
		{ value: 'plan', label: 'Plan', name: 'Invoice sync plan', content: plan },
		{
			value: 'incident',
			label: 'Incident report',
			name: 'Worker pool incident',
			content: incident
		},
		{ value: 'elements', label: 'Every element', name: 'Markdown elements', content: elements }
	]

	type Surface = 'session' | 'shared' | 'export'
	type View = { sample: string; surface: Surface; panelWidth: number }

	const design = new PersistedState<LabSettings>(SETTINGS_KEY, {
		...DEFAULT_SETTINGS
	})
	// An object stored before a setting existed lacks that setting.
	design.current = { ...DEFAULT_SETTINGS, ...design.current }
	const view = new PersistedState<View>('wm-artifact-lab-view-v1', {
		sample: 'plan',
		surface: 'session',
		panelWidth: 560
	})

	let s = $derived(design.current)
	let v = $derived(view.current)
	let sample = $derived(SAMPLES.find((x) => x.value === v.sample) ?? SAMPLES[0])

	// `:root` outranks the document's own `.wm-doc` rules, and still matches in the PDF download's
	// print frame, which copies this page's stylesheets.
	let inAppCss = $derived(labCss(s, ':root .wm-doc'))
	let exportCss = $derived(labCss(s, '.wm-doc'))
	let pageFontLinks = $derived(enabled ? fontLinks(s) : [])
	// Every face the app ships, as a download embeds them.
	let shippedFontCss = $state('')
	if (enabled) artifactFontCss().then((css) => (shippedFontCss = css))
	let exportHtml = $derived(
		artifactHtmlDocument(sample.name, sample.content, {
			css: exportCss,
			fontLinks: pageFontLinks,
			fontCss: shippedFontCss
		})
	)

	const textFonts = FONTS.filter((f) => f.kind !== 'mono').map((f) => ({
		value: f.id,
		label: f.label,
		subtitle: `${f.kind === 'serif' ? 'Serif' : 'Sans-serif'}${f.bundled ? ', ships with the app' : ''}`
	}))
	const monoFonts = FONTS.filter((f) => f.kind === 'mono').map((f) => ({
		value: f.id,
		label: f.label,
		subtitle: f.bundled ? 'Ships with the app' : undefined
	}))
	const interStyles = INTER_STYLES.map((x) => ({
		value: x.id,
		label: x.label,
		subtitle: x.subtitle
	}))
	const presetItems = PRESETS.map((p) => ({ value: p.id, label: p.label }))
	const panelWidths = [
		{ value: 420, label: 'Narrow panel (420px)' },
		{ value: 560, label: 'Panel (560px)' },
		{ value: 720, label: 'Wide panel (720px)' }
	]
	let panelWidthItems = $derived(
		panelWidths.some((w) => w.value === v.panelWidth)
			? panelWidths
			: [...panelWidths, { value: v.panelWidth, label: `Dragged (${v.panelWidth}px)` }]
	)

	function startPanelResize(e: PointerEvent) {
		e.preventDefault()
		const handle = e.currentTarget as HTMLElement
		const panel = handle.previousElementSibling as HTMLElement
		const stage = getComputedStyle(handle.parentElement!)
		const maxWidth =
			handle.parentElement!.clientWidth -
			parseFloat(stage.paddingLeft) -
			parseFloat(stage.paddingRight) -
			handle.offsetWidth
		const startX = e.clientX
		const startWidth = panel.getBoundingClientRect().width
		handle.setPointerCapture(e.pointerId)
		const onMove = (ev: PointerEvent) => {
			const width = Math.min(maxWidth, Math.max(320, startWidth + ev.clientX - startX))
			v.panelWidth = Math.round(width)
		}
		const stop = () => {
			handle.removeEventListener('pointermove', onMove)
			handle.removeEventListener('pointerup', stop)
			handle.removeEventListener('pointercancel', stop)
		}
		handle.addEventListener('pointermove', onMove)
		handle.addEventListener('pointerup', stop)
		handle.addEventListener('pointercancel', stop)
	}

	let presetId = $derived(
		PRESETS.find(
			(p) =>
				p.headingFont === s.headingFont && p.bodyFont === s.bodyFont && p.monoFont === s.monoFont
		)?.id
	)

	function applyPreset(id: string | undefined) {
		const preset = PRESETS.find((p) => p.id === id)
		if (!preset) return
		const { headingFont, bodyFont, monoFont, headingWeight } = preset
		design.current = { ...design.current, headingFont, bodyFont, monoFont, headingWeight }
	}

	// A style element built in script: svelte-preprocess would take a literal one in the markup,
	// even inside an expression, for the component's own stylesheet.
	function stylesheet(css: string): Attachment {
		return () => {
			const style = document.createElement('style')
			style.textContent = css
			document.head.append(style)
			return () => style.remove()
		}
	}
</script>

<svelte:head>
	{#each pageFontLinks as href (href)}
		<link rel="stylesheet" {href} />
	{/each}
</svelte:head>

{#snippet field(label: string)}
	<span class="text-xs font-semibold text-emphasis">{label}</span>
{/snippet}

{#if enabled}
	<div class="flex h-screen bg-surface-secondary text-primary" {@attach stylesheet(inAppCss)}>
		<aside class="flex w-80 shrink-0 flex-col gap-6 overflow-y-auto border-r bg-surface p-5">
			<div class="flex flex-col gap-1">
				<h1 class="text-lg font-semibold text-emphasis">Artifact document</h1>
				<p class="text-xs text-secondary">
					The real renderer, HTML export and PDF download, restyled live. Inter and JetBrains Mono
					ship with the app; the other fonts load from Google Fonts, on this page only.
				</p>
			</div>

			<section class="flex flex-col gap-3">
				<h2 class="text-sm font-semibold text-emphasis">Fonts</h2>
				<div class="flex flex-col gap-1">
					{@render field('Pairing')}
					<Select
						items={presetItems}
						placeholder="Custom pairing"
						bind:value={() => presetId, applyPreset}
					/>
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Headings')}
					<Select items={textFonts} bind:value={s.headingFont} />
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Body')}
					<Select items={textFonts} bind:value={s.bodyFont} />
				</div>
				{#if s.bodyFont === 'inter'}
					<div class="flex flex-col gap-1">
						{@render field('Inter style')}
						<Select items={interStyles} bind:value={s.interStyle} />
					</div>
				{/if}
				<div class="flex flex-col gap-1">
					{@render field('Code')}
					<Select items={monoFonts} bind:value={s.monoFont} />
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Heading weight')}
					<ToggleButtonGroup
						bind:selected={() => String(s.headingWeight), (w) => (s.headingWeight = Number(w))}
					>
						{#snippet children({ item })}
							{#each ['400', '500', '600', '700'] as weight (weight)}
								<ToggleButton {item} value={weight} label={weight} size="sm" />
							{/each}
						{/snippet}
					</ToggleButtonGroup>
				</div>
			</section>

			<section class="flex flex-col gap-3">
				<h2 class="text-sm font-semibold text-emphasis">Size and rhythm</h2>
				<LabSlider
					label="Text size"
					min={13}
					max={18}
					step={0.5}
					bind:value={s.size}
					format={(x) => `${x}px`}
				/>
				<LabSlider
					label="Line height"
					min={1.4}
					max={2}
					step={0.05}
					bind:value={s.leading}
					format={(x) => x.toFixed(2)}
				/>
				<LabSlider
					label="Heading scale"
					min={1.1}
					max={1.4}
					step={0.02}
					bind:value={s.scale}
					format={(x) => `${x.toFixed(2)}×`}
				/>
				<LabSlider
					label="Heading x-height"
					min={0.4}
					max={0.6}
					step={0.01}
					bind:value={s.headingAdjust}
					format={(x) => `${x.toFixed(2)} of the size`}
				/>
				<LabSlider
					label="Column width"
					min={28}
					max={56}
					step={1}
					bind:value={s.column}
					format={(x) => `${x}em, ${Math.round(x * s.size)}px`}
				/>
			</section>

			<section class="flex flex-col gap-3">
				<h2 class="text-sm font-semibold text-emphasis">Details</h2>
				<div class="flex flex-col gap-1">
					{@render field('Section headings')}
					<ToggleButtonGroup bind:selected={s.headingMarker} wrap>
						{#snippet children({ item })}
							<ToggleButton {item} value="number" label="Numbered" size="sm" />
							<ToggleButton {item} value="tick" label="Tick" size="sm" />
							<ToggleButton {item} value="rule" label="Rule" size="sm" />
							<ToggleButton {item} value="none" label="Plain" size="sm" />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Tables')}
					<ToggleButtonGroup bind:selected={s.tables}>
						{#snippet children({ item })}
							<ToggleButton {item} value="hairline" label="Hairlines" size="sm" />
							<ToggleButton {item} value="zebra" label="Striped" size="sm" />
							<ToggleButton {item} value="grid" label="Grid" size="sm" />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Dividers')}
					<ToggleButtonGroup bind:selected={s.divider}>
						{#snippet children({ item })}
							<ToggleButton {item} value="dots" label="Dots" size="sm" />
							<ToggleButton {item} value="line" label="Line" size="sm" />
							<ToggleButton {item} value="mark" label="Windmill mark" size="sm" />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<div class="flex flex-col gap-1">
					{@render field('Callouts')}
					<ToggleButtonGroup bind:selected={s.callouts}>
						{#snippet children({ item })}
							<ToggleButton {item} value="soft" label="Soft panel" size="sm" />
							<ToggleButton {item} value="bar" label="Side bar" size="sm" />
						{/snippet}
					</ToggleButtonGroup>
				</div>
			</section>

			<section class="flex flex-col gap-2">
				<h2 class="text-sm font-semibold text-emphasis">CSS for these settings</h2>
				<p class="text-xs text-secondary">
					Scoped to <code>.wm-doc</code>, ready to fold into <code>artifactDocument.css</code>.
				</p>
				<pre
					class="max-h-72 overflow-auto rounded-md bg-surface-secondary p-2 font-mono text-2xs text-primary"
					>{exportCss}</pre
				>
				<div class="flex gap-2">
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: Copy }}
						onClick={() => copyToClipboard(exportCss)}
					>
						Copy CSS
					</Button>
					<Button
						unifiedSize="sm"
						variant="subtle"
						startIcon={{ icon: RotateCcw }}
						onClick={() => (design.current = { ...DEFAULT_SETTINGS })}
					>
						Reset to the defaults
					</Button>
				</div>
			</section>
		</aside>

		<main class="flex min-w-0 flex-1 flex-col">
			<div class="flex flex-wrap items-center gap-3 border-b bg-surface px-5 py-2.5">
				<ToggleButtonGroup bind:selected={v.surface} noWFull>
					{#snippet children({ item })}
						<ToggleButton {item} value="session" label="Session panel" size="sm" />
						<ToggleButton {item} value="shared" label="Shared page" size="sm" />
						<ToggleButton {item} value="export" label="HTML export" size="sm" />
					{/snippet}
				</ToggleButtonGroup>
				<div class="w-44">
					<Select items={SAMPLES} bind:value={v.sample} />
				</div>
				{#if v.surface === 'session'}
					<div class="w-52">
						<Select items={panelWidthItems} bind:value={v.panelWidth} />
					</div>
				{/if}
				<div class="ml-auto flex items-center gap-2">
					<DarkModeToggle forcedDarkMode={false} />
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: FileDown }}
						onClick={() => printArtifact(sample.name, sample.content)}
					>
						Download as PDF
					</Button>
				</div>
			</div>

			<div class="flex min-h-0 flex-1 p-6">
				{#if v.surface === 'session'}
					<!-- The session viewer's own chrome (ArtifactViewer.svelte), minus its buttons. -->
					<div
						class="flex h-full max-w-full flex-col overflow-hidden rounded-md border bg-surface-tertiary shadow-sm"
						style:width="{v.panelWidth}px"
					>
						<div class="flex items-center gap-1.5 px-8 py-2">
							<FileText size={14} class="shrink-0 text-secondary" />
							<span class="truncate text-xs font-normal text-emphasis">{sample.name}</span>
						</div>
						<div class="min-h-0 flex-1 overflow-auto px-8">
							<ArtifactBody content={sample.content} source={false} />
						</div>
					</div>
					<div
						role="separator"
						aria-orientation="vertical"
						aria-label="Resize the panel"
						title="Drag to resize"
						class="w-1.5 shrink-0 cursor-col-resize rounded-full hover:bg-surface-hover"
						onpointerdown={startPanelResize}
					></div>
				{:else if v.surface === 'shared'}
					<!-- The body of /shared_artifacts/[id], under the app's page header. -->
					<div class="h-full w-full overflow-auto rounded-md border bg-surface-tertiary px-8">
						<ArtifactBody content={sample.content} source={false} />
					</div>
				{:else}
					<!-- Always light, like the downloaded file. -->
					<iframe
						title="Export preview"
						srcdoc={exportHtml}
						sandbox=""
						class="h-full w-[820px] max-w-full rounded-md border bg-white shadow-sm"
					></iframe>
				{/if}
			</div>
		</main>
	</div>
{:else}
	<p class="p-6 text-sm text-secondary">This page is only available in development builds.</p>
{/if}
