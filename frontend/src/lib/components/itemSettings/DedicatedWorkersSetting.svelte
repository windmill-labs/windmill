<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		dedicatedWorker: boolean | undefined
		/** Why this item cannot run on dedicated workers, e.g. its language. */
		unsupportedReason?: string
	}

	let { kind, dedicatedWorker = $bindable(), unsupportedReason }: Props = $props()
</script>

{#snippet reason()}
	{unsupportedReason}
{/snippet}

<!-- Not available on the cloud, where the toggle could only ever be disabled. -->
{#if !isCloudHosted()}
	<SettingToggle
		setting="dedicated-worker"
		label="Dedicated workers"
		tooltip={kind === 'flow'
			? 'The steps of the flow run on workers dedicated to it, which keep them warm so there is no cold start.'
			: 'The script runs on workers dedicated to it, which keep it warm so there is no cold start. A dedicated worker can reach more than 1500 runs per second.'}
		documentationLink="https://www.windmill.dev/docs/core_concepts/dedicated_workers"
		eeOnly
		disabled={!$enterpriseLicense || (!!unsupportedReason && !dedicatedWorker)}
		checked={!!dedicatedWorker}
		onchange={(on) => (dedicatedWorker = on || undefined)}
		hint={unsupportedReason ? reason : undefined}
	>
		<Alert type="info" title="Requires a dedicated worker group" size="xs">
			A worker group must listen to this {kind}. Select it in the dedicated workers section of the
			worker group configuration.
		</Alert>
	</SettingToggle>
{/if}
