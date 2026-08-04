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
	import { DEV_WORKSPACE_LOCK_RULE_NAME } from '$lib/workspaceProtectionRules.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'

	let rules: ProtectionRuleset[] | undefined = $state<ProtectionRuleset[] | undefined>(undefined)
	let selectedRule: ProtectionRuleset | undefined = $state(undefined)
	let ruleDrawer: Drawer | undefined = $state(undefined)

	// A failed load still yields an empty list so the table renders, so the deep link below has to be
	// told apart from a genuinely absent rule.
	let loadFailed = $state(false)

	async function loadRules() {
		if (!$workspaceStore) return

		try {
			rules = await WorkspaceService.listProtectionRules({ workspace: $workspaceStore })
			loadFailed = false
		} catch (error) {
			console.error('Failed to load protection rules:', error)
			sendUserToast('Failed to load protection rules', true)
			rules = []
			loadFailed = true
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadRules())
		}
	})

	// `?rule=<name>` deep-links straight into a rule's drawer, so the dev-workspace panel can point at
	// the ruleset enforcing its locks instead of dropping the reader on the list. The param is consumed
	// on open: leaving it set would re-open the drawer on every later save or tab switch, since the
	// sidebar carries the whole query string across tabs.
	$effect(() => {
		const name = $page.url.searchParams.get('rule')
		const loaded = rules
		if (!name || !loaded) return
		untrack(() => {
			const match = loaded.find((r) => r.name === name)
			if (match) {
				selectedRule = match
				ruleDrawer?.openDrawer()
			} else if (!loadFailed) {
				sendUserToast(`Protection rule '${name}' not found in this workspace`, true)
			}
			const params = new URLSearchParams(window.location.search)
			params.delete('rule')
			goto(`?${params.toString()}`, { replaceState: true, noScroll: true, keepFocus: true })
		})
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
								{#if rule.name === DEV_WORKSPACE_LOCK_RULE_NAME}
									<span class="text-2xs text-secondary">
										Applied by the dev workspace pairing. Detach the dev workspace to remove it.
									</span>
								{/if}
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
									// The reserved rule is removed by detaching the dev workspace; the API refuses
									// to delete it by name, so offering the action here could only ever fail.
									...(rule.name === DEV_WORKSPACE_LOCK_RULE_NAME
										? []
										: [
												{
													displayName: 'Delete',
													icon: Trash,
													type: 'delete' as const,
													action: async () => {
														await deleteRule(rule.name)
													}
												}
											])
								]}
							/>
						</Cell>
					</Row>
				{/each}
			{/if}
		</tbody>
	</DataTable>
</div>
