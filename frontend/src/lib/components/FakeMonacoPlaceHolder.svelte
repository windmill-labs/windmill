<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code?: string
	}

	let { code }: Props = $props()

	let lines = $derived(code?.split('\n') ?? [])

	let [clientWidth, clientHeight] = $state([0, 0])
</script>

<div
	bind:clientWidth
	bind:clientHeight
	class="h-full w-full relative editor dark:bg-[#272D38]"
	style="--vscode-editorCodeLens-lineHeight: 18px; --vscode-editorCodeLens-fontSize: 12px; --vscode-editorCodeLens-fontFeatureSettings: &quot;liga&quot; off, &quot;calt&quot; off; --code-editorInlayHintsFontFamily: Menlo, Monaco, 'Courier New', monospace;"
>
	<div
		class="monaco-editor no-user-select mac standalone showUnused showDeprecated vs-dark"
		role="code"
	>
		<div class="overflow-guard" style="width: {clientWidth}px; height: {clientHeight}px;">
			<div
				class="margin"
				role="presentation"
				aria-hidden="true"
				style="position: absolute; transform: translate3d(0px, 0px, 0px); contain: strict; top: 0px; width: {clientWidth}px; height: {clientHeight}px;"
			>
				<div
					class="margin-view-overlays"
					role="presentation"
					aria-hidden="true"
					style="position: absolute; font-family: Menlo, Monaco, &quot;Courier New&quot;, monospace; font-weight: normal; font-size: 14px; font-feature-settings: &quot;liga&quot; 0, &quot;calt&quot; 0; font-variation-settings: normal; line-height: 21px; letter-spacing: 0px; width: 51px; height: 4893px;"
				>
					{#each lines as _, i}
						<div style="top:{21 * i}px;height:21px;">
							<div class="line-numbers" style="left:0px;width:25px;">{i + 1}</div>
						</div>
					{/each}
				</div>
			</div>
			<div
				class="monaco-scrollable-element editor-scrollable vs-dark mac"
				style="position: absolute; overflow: hidden; left: 51px; width: {clientWidth}px; height: {clientHeight}px"
			>
				<div
					class="lines-content monaco-editor-background"
					style="position: absolute; overflow: hidden; width: 1.67772e+07px; height: 1.67772e+07px; transform: translate3d(0px, 0px, 0px); contain: strict; top: 0px; left: 0px;"
				>
					<div
						class="view-lines monaco-mouse-cursor-text text-tertiary/60"
						style="line-height: 21px; position: absolute; font-family: Menlo, Monaco, &quot;Courier New&quot;, monospace; font-weight: normal; font-size: 14px; font-feature-settings: &quot;liga&quot; 0, &quot;calt&quot; 0; font-variation-settings: normal; line-height: 21px; letter-spacing: 0px; width: 1143px; height: 789px;"
					>
						{#each lines as line, i}
							<div style="height: 21px; top: {i * 21}px;" class="text-nowrap whitespace-pre">
								{line}
							</div>
						{/each}
					</div>
				</div>
			</div>
		</div>
	</div>
</div>
