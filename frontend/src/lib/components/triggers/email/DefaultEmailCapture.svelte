<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base32 } from 'rfc4648'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import { fade } from 'svelte/transition'

	interface Props {
		isFlow?: boolean
		path: string
		emailDomain?: string | null
		captureInfo?: CaptureInfo | undefined
		hasPreprocessor?: boolean
		captureLoading?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		isFlow = false,
		path,
		emailDomain = null,
		captureInfo = undefined,
		hasPreprocessor = false,
		captureLoading = false,
		oncaptureToggle = undefined,
		onapplyArgs = undefined,
		onupdateSchema = undefined,
		onaddPreprocessor = undefined,
		ontestWithArgs = undefined
	}: Props = $props()

	function getCaptureEmail() {
		const cleanedPath = path.replaceAll('/', '.')
		const plainPrefix = `capture+${$workspaceStore}+${(isFlow ? 'flow.' : '') + cleanedPath}`
		const encodedPrefix = base32
			.stringify(new TextEncoder().encode(plainPrefix), {
				pad: false
			})
			.toLowerCase()
		return `${encodedPrefix}@${emailDomain}`
	}

	let captureEmail = $derived(getCaptureEmail())
</script>

{#if captureInfo}
	<CaptureSection
		captureType="default_email"
		disabled={false}
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
