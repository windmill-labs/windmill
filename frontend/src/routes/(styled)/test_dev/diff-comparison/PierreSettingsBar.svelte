<script lang="ts">
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { canUseWASM } from '@pierre/diffs'
	import { THEMES, type PierreSettings } from './pierreSettings.svelte'

	let { settings = $bindable() }: { settings: PierreSettings } = $props()

	const items = (values: string[]) => values.map((value) => ({ value, label: value }))
	const wasm = canUseWASM()
</script>

<div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs">
	<label class="flex items-center gap-1">
		Layout
		<Select size="sm" items={items(['unified', 'split'])} bind:value={settings.diffStyle} />
	</label>
	<label class="flex items-center gap-1">
		Indicators
		<Select
			size="sm"
			items={items(['bars', 'classic', 'none'])}
			bind:value={settings.diffIndicators}
		/>
	</label>
	<label class="flex items-center gap-1">
		Inline diff
		<Select
			size="sm"
			items={items(['word-alt', 'word', 'word-line', 'char', 'none'])}
			bind:value={settings.lineDiffType}
		/>
	</label>
	<label class="flex items-center gap-1">
		Hunk separators
		<Select
			size="sm"
			items={items(['line-info', 'line-info-basic', 'metadata', 'simple'])}
			bind:value={settings.hunkSeparators}
		/>
	</label>
	<label class="flex items-center gap-1">
		Highlighter
		<Select
			size="sm"
			items={[
				{ value: 'shiki-js', label: 'shiki-js' },
				{ value: 'shiki-wasm', label: 'shiki-wasm', disabled: !wasm },
				{ value: 'highlights', label: 'highlights (beta)', disabled: !wasm }
			]}
			bind:value={settings.highlighter}
		/>
	</label>
	<label class="flex items-center gap-1">
		Theme
		<Select size="sm" items={items(Object.keys(THEMES))} bind:value={settings.theme} />
	</label>
	<Toggle size="xs" bind:checked={settings.useWorkers} options={{ right: 'Worker pool' }} />
	<Toggle size="xs" bind:checked={settings.wrap} options={{ right: 'Wrap' }} />
	<Toggle size="xs" bind:checked={settings.expandUnchanged} options={{ right: 'Expand all' }} />
	<Toggle
		size="xs"
		bind:checked={settings.disableBackground}
		options={{ right: 'No line background' }}
	/>
	<Toggle
		size="xs"
		bind:checked={settings.disableLineNumbers}
		options={{ right: 'No line numbers' }}
	/>
</div>
