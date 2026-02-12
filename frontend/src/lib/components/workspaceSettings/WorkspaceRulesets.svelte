<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import RulesetEditor from './RulesetEditor.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Pen, Trash } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { WorkspaceService, type ProtectionRuleset } from '$lib/gen'

	let rules: ProtectionRuleset[] | undefined = $state<ProtectionRuleset[] | undefined>(undefined)
	let selectedRule: ProtectionRuleset | undefined = $state(undefined)
	let ruleDrawer: Drawer | undefined = $state(undefined)

	async function loadRules() {
		if (!$workspaceStore) return

		try {
			rules = await WorkspaceService.listProtectionRules({ workspace: $workspaceStore })
		} catch (error) {
			console.error('Failed to load protection rules:', error)
			sendUserToast('Failed to load protection rules', true)
			rules = []
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadRules())
		}
	})

	async function deleteRule(name: string) {
		if (!$workspaceStore) return
		try {
			await WorkspaceService.deleteProtectionRule({
				workspace: $workspaceStore,
				ruleName: name
			})
			await loadRules()
			sendUserToast('Protection rule deleted')
		} catch (error) {
			console.error('Failed to delete protection rule:', error)
			sendUserToast('Failed to delete protection rule', true)
		}
	}

	function getScopeSummary(bypassGroups: string[], bypassUsers: string[]): string {
		const groupCount = bypassGroups.length
		const userCount = bypassUsers.length
		const parts: string[] = []
		if (groupCount > 0) parts.push(`${groupCount} group${groupCount !== 1 ? 's' : ''}`)
		if (userCount > 0) parts.push(`${userCount} user${userCount !== 1 ? 's' : ''}`)
		return parts.length > 0 ? `${parts.join(', ')} can bypass` : 'No bypassers'
	}

	function getEnabledRulesCount(ruleConfig: ProtectionRuleset['rules']): number {
		return ruleConfig.length
	}

	const existingRuleNames = $derived(
		rules?.filter((r) => r.name !== selectedRule?.name).map((r) => r.name) ?? []
	)
</script>

<Drawer bind:this={ruleDrawer}>
	<DrawerContent
		title={selectedRule ? `Protection Rule: ${selectedRule.name}` : 'New Protection Rule'}
		on:close={ruleDrawer?.closeDrawer}
	>
		<RulesetEditor
			rule={selectedRule}
			existingNames={existingRuleNames}
			onUpdate={() => {
				loadRules()
				ruleDrawer?.closeDrawer()
			}}
		/>
	</DrawerContent>
</Drawer>

{#if !$enterpriseLicense}
	<Alert type="warning" title="Workspace Protection Rules is an EE feature">
		Workspace Protection Rules is a Windmill Enterprise Edition feature. It enables granular
		governance and security policies scoped to specific groups and users.
	</Alert>
	<div class="pb-4"></div>
{/if}

<div class="flex flex-row justify-between items-center mb-4">
	<div class="text-xs font-semibold text-emphasis">Protection Rules</div>
	<Button
		unifiedSize="md"
		variant="accent"
		startIcon={{ icon: Plus }}
		on:click={() => {
			selectedRule = undefined
			ruleDrawer?.openDrawer()
		}}
	>
		New rule
	</Button>
</div>

<div class="relative mb-20">
	<DataTable containerClass="bg-surface-tertiary">
		<Head>
			<tr>
				<Cell head first>Name</Cell>
				<Cell head>Bypassers</Cell>
				<Cell head>Rules</Cell>
				<Cell head last />
			</tr>
		</Head>
		<tbody class="divide-y">
			{#if rules === undefined}
				{#each new Array(3) as _}
					<tr>
						<td colspan="4">
							<Skeleton layout={[[2]]} />
						</td>
					</tr>
				{/each}
			{:else if rules.length === 0}
				<tr>
					<Cell first last colspan={4}>
						<div class="text-center py-8 text-secondary text-sm">
							No protection rules created yet. Click "New rule" to create your first rule.
						</div>
					</Cell>
				</tr>
			{:else}
				{#each rules as rule (rule.name)}
					<Row
						hoverable
						on:click={() => {
							selectedRule = rule
							ruleDrawer?.openDrawer()
						}}
					>
						<Cell first>
							<div class="flex flex-col">
								<span class="text-emphasis text-xs font-semibold">{rule.name}</span>
							</div>
						</Cell>
						<Cell>
							<span class="text-xs text-secondary"
								>{getScopeSummary(rule.bypass_groups, rule.bypass_users)}</span
							>
						</Cell>
						<Cell>
							<span class="text-xs text-secondary">
								{getEnabledRulesCount(rule.rules)} enabled
							</span>
						</Cell>
						<Cell last>
							<Dropdown
								items={[
									{
										displayName: 'Edit rule',
										icon: Pen,
										action: (e) => {
											e?.stopPropagation()
											selectedRule = rule
											ruleDrawer?.openDrawer()
										}
									},
									{
										displayName: 'Delete',
										icon: Trash,
										type: 'delete',
										action: async () => {
											await deleteRule(rule.name)
										}
									}
								]}
							/>
						</Cell>
					</Row>
				{/each}
			{/if}
		</tbody>
	</DataTable>
</div>
