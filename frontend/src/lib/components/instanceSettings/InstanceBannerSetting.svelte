<script lang="ts">
	import type { Writable } from 'svelte/store'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { classes as alertClasses, icons as alertIcons } from '../common/alert/model'
	import {
		INSTANCE_BANNER_MESSAGE_MAX_LEN,
		INSTANCE_BANNER_SETTING,
		isHttpUrl,
		resolveInstanceBanner,
		type InstanceBanner,
		type InstanceBannerSeverity
	} from '../instanceBanner'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	// Writes go through `$values[...]` member assignments: only those push the nested
	// mutation back into the store, which is what the unsaved-changes check and the Save
	// button watch. Reads go through a *snapshot* because the form mutates that object in
	// place — a derived returning the object itself keeps its identity across edits, and
	// Svelte then stops propagating to whatever depends on it. Every getter below applies
	// the same default `resolveInstanceBanner` does, so a partial stored value (config
	// sync, or one written before a field existed) displays as it will actually render.
	let banner: InstanceBanner = $derived(
		$state.snapshot($values[INSTANCE_BANNER_SETTING] ?? {}) as InstanceBanner
	)

	const severities: { value: InstanceBannerSeverity; label: string }[] = [
		{ value: 'info', label: 'Info' },
		{ value: 'warning', label: 'Warning' },
		{ value: 'error', label: 'Critical' }
	]

	// Runs the resolver the banner itself uses, so this shows what the instance gets —
	// including the "nothing is shown" cases (disabled, or an empty message).
	let preview = $derived(resolveInstanceBanner(banner))
	let linkError = $derived(
		!!banner.link?.trim() && !isHttpUrl(banner.link.trim())
			? 'Link must be an absolute http(s) URL'
			: undefined
	)
</script>

<div class="flex flex-col gap-3">
	<Toggle
		{disabled}
		bind:checked={
			() => banner.enabled === true, (v) => ($values[INSTANCE_BANNER_SETTING].enabled = v)
		}
		options={{ right: 'Show the banner to every user' }}
	/>

	<div class="flex flex-col gap-1">
		<span class="text-secondary text-xs">Message</span>
		<TextInput
			underlyingInputEl="textarea"
			size="sm"
			class="min-h-14 resize-y"
			inputProps={{
				disabled,
				rows: 2,
				maxlength: INSTANCE_BANNER_MESSAGE_MAX_LEN,
				placeholder: 'Scheduled maintenance on Saturday 12:00–14:00 UTC. Jobs may be delayed.'
			}}
			bind:value={
				() => banner.message ?? '', (v) => ($values[INSTANCE_BANNER_SETTING].message = String(v))
			}
		/>
	</div>

	<div class="flex flex-col gap-1">
		<span class="text-secondary text-xs">Severity</span>
		<ToggleButtonGroup
			{disabled}
			bind:selected={
				() => banner.severity ?? 'info', (v) => ($values[INSTANCE_BANNER_SETTING].severity = v)
			}
		>
			{#snippet children({ item })}
				{#each severities as severity (severity.value)}
					<ToggleButton value={severity.value} label={severity.label} {item} />
				{/each}
			{/snippet}
		</ToggleButtonGroup>
	</div>

	<div class="flex flex-col gap-1">
		<span class="text-secondary text-xs">Link (optional)</span>
		<div class="flex flex-col sm:flex-row gap-2">
			<TextInput
				size="sm"
				inputProps={{ type: 'text', disabled, placeholder: 'https://status.windmill.dev' }}
				error={linkError}
				bind:value={
					() => banner.link ?? '', (v) => ($values[INSTANCE_BANNER_SETTING].link = String(v))
				}
			/>
			<TextInput
				size="sm"
				inputProps={{ type: 'text', disabled, placeholder: 'Learn more' }}
				bind:value={
					() => banner.link_label ?? '',
					(v) => ($values[INSTANCE_BANNER_SETTING].link_label = String(v))
				}
			/>
		</div>
		{#if linkError}
			<span class="text-red-600 dark:text-red-400 text-xs">{linkError}</span>
		{/if}
	</div>

	<Toggle
		{disabled}
		bind:checked={
			() => banner.dismissible !== false, (v) => ($values[INSTANCE_BANNER_SETTING].dismissible = v)
		}
		options={{
			right: 'Let users dismiss it',
			rightTooltip:
				'Dismissal is remembered per browser and only for this exact announcement: editing the message, severity or link brings it back for everyone.'
		}}
	/>

	<div class="flex flex-col gap-1">
		<span class="text-secondary text-xs">Preview</span>
		{#if preview}
			{@const palette = alertClasses[preview.severity]}
			{@const Icon = alertIcons[preview.severity]}
			<div
				class="px-4 py-1.5 rounded-md flex items-center justify-center gap-3 flex-wrap {palette.bgClass}"
			>
				<Icon size={16} class="shrink-0 {palette.iconClass}" />
				<span class="text-xs font-medium {palette.titleClass}">{preview.message}</span>
				{#if preview.link}
					<span class="text-xs underline {palette.titleClass}">{preview.linkLabel}</span>
				{/if}
			</div>
		{:else}
			<span class="text-tertiary text-xs">
				{banner.enabled === true
					? 'Nothing is shown until the message is filled in.'
					: 'Nothing is shown while the banner is off.'}
			</span>
		{/if}
	</div>
</div>
