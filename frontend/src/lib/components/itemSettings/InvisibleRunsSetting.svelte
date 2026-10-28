<script lang="ts">
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		visibleToRunnerOnly: boolean | undefined
	}

	let { kind, visibleToRunnerOnly = $bindable() }: Props = $props()
</script>

<SettingToggle
	setting="invisible-runs"
	label="Make runs invisible to others"
	tooltip={`Manual runs of this ${kind} are visible only to the user who ran them, owners included. Whoever runs it can override this from the advanced menu of the run form.`}
	documentationLink="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs#invisible-runs"
	checked={!!visibleToRunnerOnly}
	onchange={(on) => (visibleToRunnerOnly = on || undefined)}
/>
