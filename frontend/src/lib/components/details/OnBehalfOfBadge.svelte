<script lang="ts">
	import { UserCog } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'

	interface Props {
		/** Authorization identity the runnable runs as: `u/{username}`, `g/{group}`, or a bare email. */
		onBehalfOf: string | undefined
		/** Address of the account `onBehalfOf` names, derived from it on the read paths. */
		onBehalfOfEmail: string | undefined
		kind: 'script' | 'flow'
	}

	let { onBehalfOf, onBehalfOfEmail, kind }: Props = $props()

	// The address is a display detail; it is only ever present alongside the identity, and
	// it repeats it when the identity is itself an email.
	let detailed = $derived(
		onBehalfOfEmail != undefined && onBehalfOfEmail !== onBehalfOf
			? `${onBehalfOf} (${onBehalfOfEmail})`
			: onBehalfOf
	)
</script>

{#if onBehalfOf}
	<Tooltip>
		{#snippet text()}
			Every run of this {kind} is permissioned as {detailed}, whoever starts it.
		{/snippet}
		<Badge color="violet" icon={{ icon: UserCog, position: 'left' }}>
			<span class="truncate max-w-40">On behalf of {onBehalfOf}</span>
		</Badge>
	</Tooltip>
{/if}
