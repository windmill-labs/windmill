<script lang="ts">
	import { Chart } from '$lib/components/chartjs-wrappers/chartJs'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import type { ChartBody } from './displayCatalog'
	import { chartConfig } from './chartConfig'

	let { chart }: { chart: ChartBody } = $props()

	const isDarkMode = useIsDarkMode()

	let container: HTMLDivElement | undefined = $state()
	// Read from the document the chart is in, not the app's: a print frame is always light.
	let theme = $state({ dark: false, surface: '#ffffff' })
	$effect(() => {
		void isDarkMode.val
		if (!container) return
		const dark = container.ownerDocument.documentElement.classList.contains('dark')
		// The gap between adjacent fills is a border in the colour of whatever surface the chart
		// sits on, which differs between run pages, chat, artifacts and print.
		let surface = dark ? '#1a1a19' : '#ffffff'
		for (let el: Element | null = container; el; el = el.parentElement) {
			const bg = getComputedStyle(el).backgroundColor
			if (bg && bg !== 'transparent' && !/rgba\(.*,\s*0\)$/.test(bg)) {
				surface = bg
				break
			}
		}
		theme = { dark, surface }
	})

	let config = $derived(chartConfig(chart, theme))
</script>

<div
	bind:this={container}
	class="relative w-full h-72 p-2"
	data-interactive
	data-chart-type={chart.type}
>
	<Chart type={config.type} data={config.data} options={config.options ?? {}} />
</div>
