<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	import Alert from '$lib/components/common/alert/Alert.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	let code = $state(JSON.stringify($app.lazyInitRequire))

	let selectedRendering = $state(
		$app.eagerRendering ? 'eager' : $app.lazyInitRequire ? 'lazy' : 'semi-lazy'
	)
</script>

<div class="flex flex-col gap-8" style="all:none;">
	<Alert type="info" title="Lazy mode for large apps performance">
		Lazy mode is a feature that allows you to lazy render components which is ideal for apps with
		many components where most are not directly visible to the user.
		<br />
		When lazy mode is enabled, components are not rendered until they are needed. This can significantly
		improve the performance of your app, especially on mobile devices.
		<br />
		Semi-Lazy mode is enabled by default, where components are initialized but are in a hidden state.
		Lazy mode is when components are not initialized at all.
	</Alert>

	<ToggleButtonGroup
		bind:selected={selectedRendering}
		on:selected={(e) => {
			if (e.detail == 'eager') {
				$app.eagerRendering = true
				$app.lazyInitRequire = undefined
			} else if (e.detail == 'semi-lazy') {
				$app.eagerRendering = undefined
				$app.lazyInitRequire = undefined
			} else {
				$app.eagerRendering = undefined
				$app.lazyInitRequire = []
				code = JSON.stringify($app.lazyInitRequire)
			}
		}}
	>
		{#snippet children({ item })}
			<ToggleButton value="eager" label="Eager" {item} />
			<ToggleButton value="semi-lazy" label="Semi-Lazy" {item} />
			<ToggleButton value="lazy" label="Lazy" {item} />
		{/snippet}
	</ToggleButtonGroup>
	{#if selectedRendering == 'eager'}
		<Section label="Eager mode">
			<span class="text-tertiary"
				>In eager mode, all components will be fully initialized, even when they are not visible.</span
			>
		</Section>
	{:else if selectedRendering == 'semi-lazy'}
		<Section label="Semi-Lazy mode">
			<span class="text-tertiary"
				>In semi-lazy mode, components will be semi-initialized when hidden. That is the default
				mode.</span
			>
		</Section>
	{:else if selectedRendering == 'lazy'}
		<Section label="Component ids to wait the initialization of before the initial refresh">
			<JsonEditor bind:value={$app.lazyInitRequire} {code} />
			<span class="text-tertiary text-xs">
				{'e.g: ["a", "b"]'}, no need to put background runnables ids
			</span>
		</Section>
	{/if}
</div>
