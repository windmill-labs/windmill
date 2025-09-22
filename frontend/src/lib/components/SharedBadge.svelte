<script lang="ts">
	import { userStore } from '$lib/stores'
	import { Users } from 'lucide-svelte'
	import Badge from './common/badge/Badge.svelte'
	import Popover from './Popover.svelte'

	interface Props {
		extraPerms?: Record<string, boolean>
		canWrite: boolean
	}

	let { extraPerms = {}, canWrite }: Props = $props()

	let { reason, kind } = $derived.by(() => {
		if ($userStore?.is_admin || $userStore?.is_super_admin) {
			return {
				reason: '',
				kind: undefined
			}
		} else {
			let kd: 'read' | 'write' | undefined = undefined
			let rson = ''
			let username = $userStore?.username ?? ''
			let pgroups = $userStore?.pgroups ?? []
			let pusername = `u/${username}`
			let extraPermsKeys = Object.keys(extraPerms ?? {})

			if (pusername in extraPermsKeys) {
				if (extraPerms?.[pusername]) {
					kd = 'write'
				} else {
					kd = 'read'
				}
				rson = 'This item was shared to you personally'
			} else {
				let writeGroup = pgroups.find((x) => extraPermsKeys.includes(x) && extraPerms?.[x])
				if (writeGroup) {
					kd = 'write'
					rson = `This item was write shared to the group ${writeGroup} which you are a member of`
				} else {
					let readGroup = pgroups.find((x) => extraPermsKeys.includes(x))
					if (readGroup) {
						kd = 'read'
						rson = `This item was read-only shared to the group ${readGroup} which you are a member of`
					} else {
						kd = undefined
					}
				}
			}
			if (kd == 'read' && canWrite) {
				kd = undefined
			}
			if (kd == undefined && !canWrite) {
				kd = 'read'
				rson = ''
			}
			return {
				reason: rson,
				kind: kd
			}
		}
	})
</script>

{#if kind === 'read' || kind === 'write'}
	<Badge capitalize color="blue" baseClass="border border-blue-200 flex gap-1 items-center">
		<Popover notClickable>
			<Users size={12} />
			{#snippet text()}
				<span>{kind == 'read' ? 'Read & Run only' : 'Read & Write'} {reason}</span>
			{/snippet}
		</Popover>
	</Badge>
{/if}
