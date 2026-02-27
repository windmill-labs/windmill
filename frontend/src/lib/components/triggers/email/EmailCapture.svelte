<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	// import { page } from '$app/state'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import { fade } from 'svelte/transition'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	interface Props {
		local_part: string | undefined
		emailDomain: string | null
		captureInfo?: CaptureInfo | undefined
		isValid?: boolean | undefined
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		local_part,
		emailDomain = null,
		captureInfo = undefined,
		isValid = undefined,
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false,
		oncaptureToggle = undefined,
		onapplyArgs = undefined,
		onupdateSchema = undefined,
		onaddPreprocessor = undefined,
		ontestWithArgs = undefined
	}: Props = $props()

	let captureEmail = $derived(`capture+${$workspaceStore}-${local_part}@${emailDomain}`)
</script>

{#if captureInfo}
	<CaptureSection
		captureType="email"
		disabled={isValid === false}
		{captureInfo}
		{captureLoading}
		oncaptureToggle={oncaptureToggle}
		onapplyArgs={onapplyArgs}
		onupdateSchema={onupdateSchema}
		onaddPreprocessor={onaddPreprocessor}
		ontestWithArgs={ontestWithArgs}
		{hasPreprocessor}
		{isFlow}
	>
		{#snippet description()}
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Send an email to the test address below to simulate an email trigger.
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to email events on this test address.
				</p>
			{/if}
		{/snippet}
		<Label label="Test email address" disabled={!captureInfo.active}>
			<ClipboardPanel content={captureEmail} disabled={!captureInfo.active} />
		</Label>
	</CaptureSection>
{/if}
