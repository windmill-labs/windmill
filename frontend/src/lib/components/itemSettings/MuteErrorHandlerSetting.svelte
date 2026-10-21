<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		muted: boolean | undefined
	}

	let { kind, muted = $bindable() }: Props = $props()
</script>

<!-- Saved by the next deploy like every other setting of the editor. The script and flow
     pages keep a toggle that mutes the deployed item at once. -->
<SettingToggle
	setting="error-handler"
	label="Mute workspace error handler"
	tooltip={`Failures of this ${kind} do not trigger the workspace error handler.`}
	documentationLink="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
	eeOnly
	disabled={!$enterpriseLicense && !muted}
	checked={!!muted}
	onchange={(on) => (muted = on)}
/>
