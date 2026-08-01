<script lang="ts">
	import { UserCog } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'

	interface Props {
		/** Authorization identity the runnable runs as: `u/{username}`, `g/{group}`, or a bare email. */
		onBehalfOf: string | undefined
		/** Address of the account `onBehalfOf` names, derived from it server-side. */
		onBehalfOfEmail: string | undefined
		kind: 'script' | 'flow'
	}

	let { onBehalfOf, onBehalfOfEmail, kind }: Props = $props()

	// Rows written before the identity half existed carry only the address, so fall back
	// to it rather than showing nothing for a runnable that does run on behalf of someone.
	let identity = $derived(onBehalfOf ?? onBehalfOfEmail)
	let detailed = $derived(
		onBehalfOfEmail != undefined && onBehalfOfEmail !== identity
			? `${identity} (${onBehalfOfEmail})`
			: identity
	)
</script>

{#if identity}
	<Tooltip>
		{#snippet text()}
			Every run of this {kind} is permissioned as {detailed}, whoever starts it.
		{/snippet}
		<Badge color="violet" icon={{ icon: UserCog, position: 'left' }}>
			<span class="truncate max-w-40">On behalf of {identity}</span>
		</Badge>
	</Tooltip>
{/if}
