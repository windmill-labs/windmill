<script lang="ts">
	import { SecondsInput } from '$lib/components/common'
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		deleteAfterSecs: number | undefined
	}

	let { kind, deleteAfterSecs = $bindable() }: Props = $props()
</script>

<SettingToggle
	setting="lifetime"
	label="Delete after use"
	tooltip={kind === 'flow'
		? 'The logs, arguments and results of every step are deleted this long after the flow completes. 0 deletes them right away. The deletion is irreversible.'
		: 'The logs, arguments and result of each run are deleted this long after it completes and returns its result. 0 deletes them right away. Only applies to synchronous webhooks and to the script run as a flow step. The deletion is irreversible.'}
	documentationLink={kind === 'script'
		? 'https://www.windmill.dev/docs/script_editor/settings#delete-after-use'
		: undefined}
	eeOnly
	disabled={!$enterpriseLicense}
	checked={deleteAfterSecs != null}
	onchange={(on) => (deleteAfterSecs = on ? 0 : undefined)}
>
	<SecondsInput bind:seconds={deleteAfterSecs} disabled={!$enterpriseLicense} />
</SettingToggle>
