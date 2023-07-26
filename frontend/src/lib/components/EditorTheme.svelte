<script lang="ts">
	import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'
	import { onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import github from 'svelte-highlight/styles/github'
	import nord from 'svelte-highlight/styles/nord'
	import { each } from 'chart.js/helpers'
	import { Chart } from 'chart.js'

	meditor.defineTheme('nord', {
		base: 'vs-dark',
		inherit: true,
		rules: [
			{
				background: '2E3440',
				token: ''
			},
			{
				foreground: '616e88',
				token: 'comment'
			},
			{
				foreground: 'a3be8c',
				token: 'string'
			},
			{
				foreground: 'b48ead',
				token: 'constant.numeric'
			},
			{
				foreground: '81a1c1',
				token: 'constant.language'
			},
			{
				foreground: '81a1c1',
				token: 'keyword'
			},
			{
				foreground: '81a1c1',
				token: 'storage'
			},
			{
				foreground: '81a1c1',
				token: 'storage.type'
			},
			{
				foreground: '8fbcbb',
				token: 'entity.name.class'
			},
			{
				foreground: '8fbcbb',
				fontStyle: '  bold',
				token: 'entity.other.inherited-class'
			},
			{
				foreground: '88c0d0',
				token: 'entity.name.function'
			},
			{
				foreground: '81a1c1',
				token: 'entity.name.tag'
			},
			{
				foreground: '8fbcbb',
				token: 'entity.other.attribute-name'
			},
			{
				foreground: '88c0d0',
				token: 'support.function'
			},
			{
				foreground: 'f8f8f0',
				background: 'f92672',
				token: 'invalid'
			},
			{
				foreground: 'f8f8f0',
				background: 'ae81ff',
				token: 'invalid.deprecated'
			},
			{
				foreground: 'b48ead',
				token: 'constant.color.other.rgb-value'
			},
			{
				foreground: 'ebcb8b',
				token: 'constant.character.escape'
			},
			{
				foreground: '8fbcbb',
				token: 'variable.other.constant'
			}
		],
		colors: {
			'editor.foreground': '#D8DEE9',
			'editor.background': '#2E3440',
			'editor.selectionBackground': '#434C5ECC',
			'editor.lineHighlightBackground': '#3B4252',
			'editorCursor.foreground': '#D8DEE9',
			'editorWhitespace.foreground': '#434C5ECC'
		}
	})

	meditor.defineTheme('myTheme', {
		base: 'vs',
		inherit: true,
		rules: [],
		colors: {
			'editorLineNumber.foreground': '#999',
			'editorGutter.background': '#F9FAFB'
		}
	})

	let darkMode: boolean = false

	function onThemeChange() {
		if (document.documentElement.classList.contains('dark')) {
			meditor.setTheme('nord')
			darkMode = true

			each(Chart.instances, (instance) => {
				instance.options = {
					scales: {
						y: {
							ticks: {
								color: '#e0e7ed'
							},
							grid: {
								color: '#4a5568'
							}
						},
						x: {
							ticks: {
								color: '#e0e7ed'
							},
							grid: {
								color: '#4a5568'
							}
						}
					},
					plugins: {
						legend: {
							labels: {
								color: '#e0e7ed'
							}
						}
					}
				}
				instance.update()
			})
		} else {
			meditor.setTheme('myTheme')
			darkMode = false

			each(Chart.instances, (instance) => {
				instance.options = {
					scales: {
						y: {
							ticks: {
								color: '#4a5568'
							},
							grid: {
								color: '#e0e7ed'
							}
						},
						x: {
							ticks: {
								color: '#4a5568'
							},
							grid: {
								color: '#e0e7ed'
							}
						}
					},
					plugins: {
						legend: {
							labels: {
								color: '#4a5568'
							}
						}
					}
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
