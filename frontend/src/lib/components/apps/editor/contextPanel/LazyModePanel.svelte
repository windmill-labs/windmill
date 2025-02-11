<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import Section from '$lib/components/Section.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	let code = JSON.stringify($app.lazyInitRequire)
</script>

<div class="flex flex-col gap-8" style="all:none;">
	<Alert type="info" title="Lazy mode for large apps performance">
		Lazy mode is a feature that allows you to lazy render components which is ideal for apps with
		many components where most are not directly visible to the user.
		<br />
		When lazy mode is enabled, components are not rendered until they are needed. This can significantly
		improve the performance of your app, especially on mobile devices.
		<br />
		You can enable lazy mode below, but you will need to declare the list of components whose initialization
		is expected to see initialized before the initial refresh of the app happens.
	</Alert>

	<Toggle
		label="Lazy mode"
		value={Boolean($app.lazyInitRequire)}
		on:change={(e) => {
			$app.lazyInitRequire = e.detail ? [] : undefined
			code = JSON.stringify($app.lazyInitRequire)
		}}
		options={{
			right: 'Lazy mode'
		}}
	/>

	<Section label="Component ids to wait the initialization of before the initial refresh">
		{#if $app.lazyInitRequire}
			<JsonEditor bind:value={$app.lazyInitRequire} {code} />
			<span class="text-tertiary text-xs">
				{'e.g: ["a", "b"]'}, no need to put background runnables ids
			</span>
		{:else}
			<span class="text-tertiary"
				>Without lazy mode, all components' initialization will be waited on the initial refresh.</span
			>
		{/if}
	</Section>
</div>
