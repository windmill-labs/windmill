<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SecondsInput } from '$lib/components/common'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		ttl: number | undefined
		ignoreS3Path: boolean | undefined
	}

	let { kind, ttl = $bindable(), ignoreS3Path = $bindable() }: Props = $props()
</script>

<SettingToggle
	setting="cache"
	label="Cache results"
	tooltip={`The result of the ${kind} is kept for each set of inputs, and a run with the same inputs returns it instead of running again.`}
	documentationLink={kind === 'flow'
		? 'https://www.windmill.dev/docs/flows/cache#cache-flows'
		: 'https://www.windmill.dev/docs/core_concepts/caching'}
	checked={ttl !== undefined}
	onchange={(on) => {
		ttl = on ? 300 : undefined
		if (!on) ignoreS3Path = undefined
	}}
>
	<Label label="How long to keep cache valid">
		<SecondsInput bind:seconds={ttl} />
	</Label>
	<Toggle
		size="2xs"
		bind:checked={() => ignoreS3Path, (v) => (ignoreS3Path = v || undefined)}
		options={{
			right: 'Ignore S3 object paths',
			rightTooltip:
				'Two S3 objects with the same content hit the same cache entry, whatever their path.'
		}}
	/>
</SettingToggle>
