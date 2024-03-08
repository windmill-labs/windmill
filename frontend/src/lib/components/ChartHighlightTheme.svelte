<script lang="ts">
	import { onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import github from 'svelte-highlight/styles/github'
	import nord from 'svelte-highlight/styles/nord'
	import { each } from 'chart.js/helpers'
	import { Chart } from 'chart.js'

	let darkMode: boolean = false

	function onThemeChange() {
		if (document.documentElement.classList.contains('dark')) {
			darkMode = true
			each(Chart.instances, (instance) => {
				if (instance.options.scales?.y?.ticks?.color !== undefined) {
					instance.options.scales.y.ticks.color = '#e0e7ed'
				}
				if (instance.options.scales?.y?.grid?.color !== undefined) {
					instance.options.scales.y.grid.color = '#4a5568'
				}
				if (instance.options.scales?.x?.ticks?.color !== undefined) {
					instance.options.scales.x.ticks.color = '#e0e7ed'
				}
				if (instance.options.scales?.x?.grid?.color !== undefined) {
					instance.options.scales.x.grid.color = '#4a5568'
				}
				if (instance.options.plugins?.legend?.labels?.color !== undefined) {
					instance.options.plugins.legend.labels.color = '#e0e7ed'
				}
				instance.update()
			})
		} else {
			darkMode = false
			each(Chart.instances, (instance) => {
				if (instance.options.scales?.y?.ticks?.color !== undefined) {
					instance.options.scales.y.ticks.color = '#4a5568'
				}
				if (instance.options.scales?.y?.grid?.color !== undefined) {
					instance.options.scales.y.grid.color = '#e0e7ed'
				}
				if (instance.options.scales?.x?.ticks?.color !== undefined) {
					instance.options.scales.x.ticks.color = '#4a5568'
				}
				if (instance.options.scales?.x?.grid?.color !== undefined) {
					instance.options.scales.x.grid.color = '#e0e7ed'
				}
				if (instance.options.plugins?.legend?.labels?.color !== undefined) {
					instance.options.plugins.legend.labels.color = '#4a5568'
				}
				instance.update()
			})
		}
	}

	onMount(() => {
		onThemeChange()
	})
</script>

<DarkModeObserver on:change={onThemeChange} />

<svelte:head>
	{#if darkMode}
		{@html nord}
	{:else}
		{@html github}
	{/if}
</svelte:head>
