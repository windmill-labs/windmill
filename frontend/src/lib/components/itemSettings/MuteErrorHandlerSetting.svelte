<script lang="ts">
	import { toggleWorkspaceErrorHandler } from '$lib/components/details/errorHandlerToggle'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		path: string
		/** Also sent with the next deploy. */
		muted: boolean | undefined
	}

	let { kind, path, muted = $bindable() }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

	// The toggle shows the click until the request settles, then the muted state it left.
	let pending: boolean | undefined = $state(undefined)

	async function toggle() {
		const next = await toggleWorkspaceErrorHandler($operatingWorkspace, kind, path, muted)
		if (next !== undefined) muted = next
		pending = undefined
	}
</script>

{#snippet eeHint()}
	Enterprise Edition only.
{/snippet}

<SettingToggle
	setting="mute-error-handler"
	label="Mute workspace error handler"
	tooltip={`Failures of this ${kind} do not trigger the workspace error handler. Applies to the deployed ${kind} right away.`}
	documentationLink="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
	bind:checked={() => pending ?? !!muted, (v) => (pending = v)}
	onchange={toggle}
	hint={$enterpriseLicense ? undefined : eeHint}
/>
