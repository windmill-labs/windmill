<script lang="ts">
	import type { ProtectionRuleset } from '$lib/gen'
	import { userStore, type UserExt } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import {
		fetchProtectionRulesForWorkspace,
		isRuleActiveInRulesets,
		canUserBypassRuleKindInRulesets,
		getActiveRulesetsForKindInRulesets
	} from '$lib/workspaceProtectionRules.svelte'
	import { Alert } from './common'
	import { resource } from 'runed'

	let {
		parentWorkspaceId,
		onUpdateCanDeploy = (value) => {}
	}: {
		parentWorkspaceId: string
		onUpdateCanDeploy?: (value: boolean) => void
	} = $props()

	let overrideChecked = $state(false)

	// Fetch the parent's rules and the user's identity in the parent together. Both must come from the
	// parent: bypass is judged with is_admin/groups in the PARENT (per-workspace), not the active/fork
	// workspace's `$userStore`; getUserExt returns undefined for a non-member, treated as no bypass.
	const parentDataResource = resource(
		() => parentWorkspaceId,
		async (wsId) => {
			if (!wsId) return { rules: [] as ProtectionRuleset[], user: undefined as UserExt | undefined }
			const [rules, user] = await Promise.all([
				fetchProtectionRulesForWorkspace(wsId),
				getUserExt(wsId)
			])
			return { rules, user }
		}
	)
	let parentRulesets = $derived(parentDataResource.current?.rules ?? [])
	let parentUserInfo = $derived(parentDataResource.current?.user)

	let activeDeployRulesets = $derived(
		getActiveRulesetsForKindInRulesets(parentRulesets, 'DisableDirectDeployment')
	)

	let canBypass = $derived(
		canUserBypassRuleKindInRulesets(parentRulesets, 'DisableDirectDeployment', parentUserInfo)
	)

	// Block deploy until the parent's rules/identity have loaded: while loading `parentRulesets` is
	// empty, which would otherwise read as "no lock" and briefly enable deploy before the real
	// lock/bypass check resolves.
	let canDeploy = $derived(
		!parentDataResource.loading &&
			(!isRuleActiveInRulesets(parentRulesets, 'DisableDirectDeployment') ||
				(canBypass && overrideChecked))
	)

	// Reset override when parent workspace changes
	$effect(() => {
		parentWorkspaceId
		overrideChecked = false
	})

	// Communicate deployment status to parent
	$effect(() => {
		onUpdateCanDeploy(canDeploy)
	})
</script>

{#if !$userStore?.operator && activeDeployRulesets.length > 0}
	<Alert type="info" title="Parent workspace protection active" class="my-2">
		<div class="flex flex-col gap-2">
			<p>
				The workspace {parentWorkspaceId} has a protection rule{activeDeployRulesets.length > 1
					? 's'
					: ''}
				<b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b>
				that restrict{activeDeployRulesets.length > 1 ? '' : 's'} direct deployments. You need to merge
				changes through the synced git repo with Git Sync, or by asking a user with the rights to bypass
				this rule.
			</p>
			{#if canBypass}
				<label class="flex items-center gap-2 cursor-pointer">
					<input class="rounded max-w-4" type="checkbox" bind:checked={overrideChecked} />
					<span class="text-xs">Bypass restriction and deploy anyway</span>
				</label>
			{/if}
		</div>
	</Alert>
{/if}
