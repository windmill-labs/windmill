<!--
@component
The Permissions section of the assistant settings modal: what the user's role lets them create
and edit in the chat's workspace, next to what the assistant can make for them from the chat.
The assistant acts with the user's own rights, but its write tools go through drafts, so for
a restricted role the second column can be narrower.
-->
<script lang="ts">
	import { Badge, Section } from '$lib/components/common'
	import { Check, Minus } from 'lucide-svelte'
	import { userWorkspaces } from '$lib/stores'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { capitalize } from '$lib/utils/capitalize'
	import { capabilitiesIn, isRestricted } from '$lib/roleCapabilities'
	import { OPERATOR_SETTINGS_LOCATION, permissionRows, roleName } from './assistantPermissions'

	let { ws }: { ws: string } = $props()

	const acting = useActingUser(() => ws)

	let user = $derived(acting.current)
	let entry = $derived($userWorkspaces.find((w) => w.id === ws))
	let caps = $derived(capabilitiesIn(user, entry))
	let restricted = $derived(isRestricted(caps))
	let role = $derived(user ? roleName(user, entry) : undefined)
	let rows = $derived(user ? permissionRows(user, caps) : [])
	let chatNarrower = $derived(rows.some((r) => r.user && !r.assistant))
</script>

<Section
	label="Permissions"
	description="What you can create and edit in this workspace, and what the assistant can make for you from this chat."
	class="flex flex-col gap-4"
>
	{#snippet action()}
		{#if role}
			<Badge color={restricted ? 'yellow' : 'gray'}>{capitalize(role)}</Badge>
		{/if}
	{/snippet}

	{#if !user}
		<p class="text-xs text-secondary">
			{acting.resolved(ws)
				? 'Your role in this workspace could not be read.'
				: 'Checking your role…'}
		</p>
	{:else}
		<div class="grid max-w-sm grid-cols-[1fr_auto_auto] gap-x-6 gap-y-1.5 text-xs">
			<span class="text-2xs text-secondary">Create and edit</span>
			<span class="text-2xs text-secondary text-center">You</span>
			<span class="text-2xs text-secondary text-center">From this chat</span>
			{#snippet cell(allowed: boolean)}
				<span class="flex justify-center">
					{#if allowed}
						<Check size={14} class="text-primary" aria-hidden="true" />
					{:else}
						<Minus size={14} class="text-hint" aria-hidden="true" />
					{/if}
					<span class="sr-only">{allowed ? 'Allowed' : 'Not allowed'}</span>
				</span>
			{/snippet}
			{#each rows as row (row.label)}
				<span class="text-primary">{row.label}</span>
				{@render cell(row.user)}
				{@render cell(row.assistant)}
			{/each}
		</div>

		<div class="flex flex-col gap-2 text-xs text-secondary">
			{#if chatNarrower}
				<p>
					The assistant acts as you, with your rights, but it saves every change as a draft before
					deploying it, and your role can't save drafts of everything you can edit. Make those on
					their own pages instead.
				</p>
			{/if}
			<p>Anyone in the workspace can run the scripts and flows they can see.</p>
			<p>
				Items in folders you can't read are hidden from you and from the assistant. Someone with
				access to the folder can run them.
			</p>
			{#if restricted}
				<p>
					A workspace admin can give your role more rights in {OPERATOR_SETTINGS_LOCATION}.
				</p>
			{/if}
		</div>
	{/if}
</Section>
