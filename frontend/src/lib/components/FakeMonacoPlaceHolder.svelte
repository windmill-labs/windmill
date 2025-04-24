<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	import { getOS } from '$lib/utils'

	type Props = {
		code?: string
		autoheight?: boolean
		lineNumbersWidth?: number
		lineNumbersOffset?: number
		class?: string
		fontSize?: number
	}

	let {
		code,
		autoheight = false,
		lineNumbersWidth = 51,
		lineNumbersOffset = 0,
		class: className = '',
		fontSize = 14
	}: Props = $props()

	// https://github.com/microsoft/vscode/blob/baa2dad3cdacd97ac02eff0604984faf1167ff1e/src/vs/editor/common/config/editorOptions.ts#L5421
	const DEFAULT_WINDOWS_FONT_FAMILY = "Consolas, 'Courier New', monospace"
	const DEFAULT_MAC_FONT_FAMILY = "Menlo, Monaco, 'Courier New', monospace"
	const DEFAULT_LINUX_FONT_FAMILY = "'Droid Sans Mono', 'monospace', monospace"
	const fontFamily =
		getOS() === 'Windows'
			? DEFAULT_WINDOWS_FONT_FAMILY
			: getOS() === 'macOS'
				? DEFAULT_MAC_FONT_FAMILY
				: DEFAULT_LINUX_FONT_FAMILY

	// https://github.com/microsoft/vscode/blob/baa2dad3cdacd97ac02eff0604984faf1167ff1e/src/vs/editor/common/config/fontInfo.ts#L14
	const GOLDEN_LINE_HEIGHT_RATIO = getOS() == 'macOS' ? 1.5 : 1.35

	let lines = $derived(code?.split('\n') ?? [])

	const charWidth = 9 // try to match as closely as possible to monaco editor

	const lineHeight = fontSize * GOLDEN_LINE_HEIGHT_RATIO

	let [clientWidth, clientHeight] = $state([0, 0])
	let showHorizontalScrollbar = $derived(
		lines.some((line) => line.length * charWidth > clientWidth - 40)
	)

	let [editorWidth, editorHeight] = $derived([
		clientWidth,
		autoheight ? lines.length * lineHeight + (showHorizontalScrollbar ? 12 : 0) : clientHeight
	])
</script>

<!-- Copy pasted from actual monaco editor in the web inspector -->

<div
	bind:clientWidth
	bind:clientHeight
	class="h-full w-full relative editor dark:bg-[#272D38] {className}"
	style="--vscode-editorCodeLens-lineHeight: 18px; --vscode-editorCodeLens-fontSize: 12px; --vscode-editorCodeLens-fontFeatureSettings: 'liga' off, 'calt' off; --code-editorInlayHintsFontFamily: {fontFamily};"
>
	<div
		class="monaco-editor no-user-select mac standalone showUnused showDeprecated vs-dark"
		role="code"
	>
		<div class="overflow-guard" style="width: {editorWidth}px; height: {editorHeight}px;">
			<div
				class="margin"
				role="presentation"
				aria-hidden="true"
				style="position: absolute; transform: translate3d(0px, 0px, 0px); contain: strict; top: 0px; width: {editorWidth}px; height: {editorHeight}px;"
			>
				<div
					class="margin-view-overlays"
					role="presentation"
					aria-hidden="true"
					style="position: absolute; font-family: {fontFamily}; font-weight: normal; font-size: {fontSize}px; font-feature-settings: 'liga' 0, 'calt' 0; font-variation-settings: normal; line-height: {lineHeight}px; letter-spacing: 0px; width: {lineNumbersWidth}px; height: 4893px;"
				>
					{#each lines as _, i}
						<div style="top:{lineHeight * i}px;height:{lineHeight}px;">
							<div class="line-numbers" style="left:{lineNumbersOffset}px;width:25px;">{i + 1}</div>
						</div>
					{/each}
				</div>
			</div>
			<div
				class="monaco-scrollable-element editor-scrollable vs-dark mac"
				style="position: absolute; overflow: hidden; left: {lineNumbersWidth}px; width: {editorWidth}px; height: {editorHeight}px"
			>
				<div
					class="lines-content monaco-editor-background"
					style="position: absolute; overflow: hidden; width: 1.67772e+07px; height: 1.67772e+07px; transform: translate3d(0px, 0px, 0px); contain: strict; top: 0px; left: 0px;"
				>
					<div
						class="view-lines monaco-mouse-cursor-text text-tertiary/60"
						style="line-height: {lineHeight}px; position: absolute; font-family: {fontFamily}; font-weight: normal; font-size: {fontSize}px; font-feature-settings: 'liga' 0, 'calt' 0; font-variation-settings: normal; line-height: {lineHeight}px; letter-spacing: 0px; width: 1143px; height: 789px;"
					>
						{#each lines as line, i}
							<div
								style="height: {lineHeight}px; top: {i * lineHeight}px;"
								class="text-nowrap whitespace-pre"
							>
								{line}
							</div>
						{/each}
					</div>
				</div>
			</div>
		</div>
	</div>
</div>
