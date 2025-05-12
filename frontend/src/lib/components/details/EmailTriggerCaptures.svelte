<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base32 } from 'rfc4648'
	import ClipboardPanel from './ClipboardPanel.svelte'
	import CaptureSection, { type CaptureInfo } from '../triggers/CaptureSection.svelte'
	import { fade } from 'svelte/transition'

	export let isFlow: boolean = false
	export let path: string
	export let emailDomain: string | null = null
	export let captureInfo: CaptureInfo | undefined = undefined
	export let hasPreprocessor: boolean = false

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

	$: captureEmail = getCaptureEmail()
</script>

{#if captureInfo}
	<CaptureSection
		captureType="email"
		disabled={false}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		{hasPreprocessor}
		{isFlow}
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Send an email to the test address below to simulate an email trigger.
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to email events on this test address.
				</p>
			{/if}
		</svelte:fragment>
		<Label label="Test email address" disabled={!captureInfo.active}>
			<ClipboardPanel content={captureEmail} disabled={!captureInfo.active} />
		</Label>
	</CaptureSection>
{/if}
