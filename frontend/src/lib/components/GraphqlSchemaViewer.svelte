<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { editor as meditor, KeyMod, KeyCode } from 'monaco-editor'
	import { editorFontSize } from '$lib/editorFontSize.svelte'
	import { preventHorizontalNavigationSwipe } from '$lib/editorUtils'

	import { onDestroy, onMount } from 'svelte'

	let divEl: HTMLDivElement | null = $state(null)
	let editor: meditor.IStandaloneCodeEditor

	interface Props {
		code?: string
		class?: string
	}

	let { code = '', class: className = '' }: Props = $props()

	async function loadMonaco() {
		editor = meditor.create(divEl as HTMLDivElement, {
			value: code,
			language: 'graphql',
			readOnly: true,
			automaticLayout: true,
			scrollBeyondLastLine: false,
			lineNumbers: 'off',
			fontSize: editorFontSize.regular,
			minimap: { enabled: false }
		})

		// In VSCode webview (iframe), clipboard operations need to use execCommand
		// because the webview has restricted clipboard API access
		if (window.parent !== window) {
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyC, function () {
				document.execCommand('copy')
			})
		}
	}

	onMount(async () => {
		if (BROWSER) {
			await loadMonaco()
		}
	})

	$effect(() => {
		const fontSize = editorFontSize.regular
		if (editor) {
			editor.updateOptions({ fontSize })
		}
	})

	onDestroy(() => {
		try {
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

<div bind:this={divEl} use:preventHorizontalNavigationSwipe class="{className} editor"></div>
