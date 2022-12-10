<script lang="ts">
	import { userStore } from '$lib/stores'
	import { Users } from 'lucide-svelte'
	import Badge from './common/badge/Badge.svelte'
	import Tooltip from './Tooltip.svelte'

	export let extraPerms: Record<string, boolean> = {}
	export let canWrite: boolean

	let kind: 'read' | 'write' | undefined = undefined
	let reason = ''

	$: {
		let username = $userStore?.username ?? ''
		let pgroups = $userStore?.pgroups ?? []
		let pusername = `u/${username}`
		let extraPermsKeys = Object.keys(extraPerms)

		if (pusername in extraPermsKeys) {
			if (extraPerms[pusername]) {
				kind = 'write'
			} else {
				kind = 'read'
			}
			reason = 'This item was shared to you personally'
		} else {
			let writeGroup = pgroups.find((x) => extraPermsKeys.includes(x) && extraPerms[x])
			if (writeGroup) {
				kind = 'write'
				reason = `This item was write shared to the group ${writeGroup} which you are a member of`
			} else {
				let readGroup = pgroups.find((x) => extraPermsKeys.includes(x))
				if (readGroup) {
					kind = 'read'
					reason = `This item was read-only shared to the group ${readGroup} which you are a member of`
				} else {
					kind = undefined
				}
			}
		}
		if (kind == 'read' && canWrite) {
			kind = undefined
		}
		if (kind == undefined && !canWrite) {
			kind = 'read'
			reason = ''
		}
	}
</script>

{#if kind === 'read' || kind === 'write'}
	<Badge capitalize color="blue" baseClass="border border-blue-200 flex gap-1 items-center">
		<Users size={12} />
		{kind}
		{#if reason}
			<Tooltip><span class="normal-case">{reason}</span></Tooltip>
		{/if}
	</Badge>
{/if}
