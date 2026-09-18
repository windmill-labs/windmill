<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		summarizeContextSection,
		type AgentContext,
		type ContextSectionId,
		type ContextTool,
		type ContextSummaryActions
	} from './contextSummary'

	let {
		ctx,
		section,
		actions,
		toggleBlockedReason
	}: {
		ctx: AgentContext
		section: ContextSectionId
		actions: ContextSummaryActions
		/** Why the switches are disabled, when they are — a session whose fork is still
		 * staged would store the choice under the parent workspace. */
		toggleBlockedReason?: string
	} = $props()

	let summary = $derived(summarizeContextSection(ctx, section))
	let expandedInstructions = $state<Record<string, boolean>>({})
</script>

{#snippet toolChips(tools: ContextTool[])}
	<div class="flex flex-wrap gap-1">
		{#each tools as tool (tool.name)}
			<span
				class="font-mono text-2xs px-1.5 py-0.5 rounded bg-surface-secondary text-secondary"
				title={tool.description}>{tool.name}</span
			>
		{/each}
	</div>
{/snippet}

{#snippet scope()}
	<span class="text-2xs text-hint">Applies to all your sessions in {ctx.workspace}</span>
{/snippet}

{#snippet footer()}
	<div class="flex justify-end">
		<Button variant="subtle" unifiedSize="xs" onclick={() => actions.onManage(section)}>
			Manage
		</Button>
	</div>
{/snippet}

<div class="flex flex-col gap-2">
	{#if summary.empty}
		<div class="flex items-center gap-2">
			<span class="text-2xs text-hint">{summary.emptyText}</span>
			{#if summary.emptyAction}
				<Button variant="default" unifiedSize="xs" onclick={() => actions.onManage(section)}>
					{summary.emptyAction}
				</Button>
			{/if}
		</div>
	{:else if section === 'tools'}
		{#each [{ key: 'read', label: 'Read', tools: ctx.tools.filter((t) => t.readOnly) }, { key: 'write', label: 'Write', tools: ctx.tools.filter((t) => !t.readOnly) }] as group (group.key)}
			{#if group.tools.length}
				<div class="flex flex-col gap-1">
					<span class="text-2xs font-semibold text-emphasis"
						>{group.label}<span class="font-normal text-hint"> · {group.tools.length}</span></span
					>
					{@render toolChips(group.tools)}
				</div>
			{/if}
		{/each}
		{@render footer()}
	{:else if section === 'skills'}
		{@render scope()}
		<div class="flex flex-col">
			{#each ctx.skills as skill (skill.path)}
				<div class="flex items-center gap-3 py-1">
					<div class="flex flex-col min-w-0 grow">
						<span class="text-xs {skill.enabled ? 'text-primary' : 'text-disabled'} truncate"
							>{skill.name}</span
						>
						{#if skill.description}
							<span class="text-2xs text-secondary truncate">{skill.description}</span>
						{/if}
					</div>
					<Toggle
						size="2xs"
						disabled={toggleBlockedReason !== undefined}
						options={{ title: toggleBlockedReason }}
						checked={skill.enabled}
						on:change={(e) => actions.onToggleSkill(skill.path, e.detail)}
					/>
				</div>
			{/each}
		</div>
		{@render footer()}
	{:else if section === 'mcp'}
		{@render scope()}
		<div class="flex flex-col">
			{#each ctx.mcpServers as server (server.path)}
				<div class="flex items-center gap-3 py-1">
					<div class="flex flex-col min-w-0 grow">
						<span class="text-xs {server.enabled ? 'text-primary' : 'text-disabled'} truncate"
							>{server.name}</span
						>
						{#if server.description}
							<span class="text-2xs text-secondary truncate">{server.description}</span>
						{/if}
					</div>
					<Toggle
						size="2xs"
						disabled={toggleBlockedReason !== undefined}
						options={{ title: toggleBlockedReason }}
						checked={server.enabled}
						on:change={(e) => actions.onToggleMcp(server.path, e.detail)}
					/>
				</div>
			{/each}
		</div>
		{@render footer()}
	{:else if section === 'instructions'}
		{#each [{ key: 'workspace', label: 'Workspace', text: ctx.instructions.workspace }, { key: 'user', label: 'Personal', text: ctx.instructions.user }] as block (block.key)}
			<div class="flex flex-col gap-0.5">
				<span class="text-2xs font-semibold text-emphasis">{block.label}</span>
				{#if block.text?.trim()}
					<p
						class="text-2xs text-secondary whitespace-pre-wrap {expandedInstructions[block.key]
							? ''
							: 'line-clamp-3'}">{block.text.trim()}</p
					>
					{#if block.text.trim().split('\n').length > 3 || block.text.length > 240}
						<button
							type="button"
							class="self-start text-2xs text-accent hover:underline"
							onclick={() => (expandedInstructions[block.key] = !expandedInstructions[block.key])}
							>{expandedInstructions[block.key] ? 'Show less' : 'Show more'}</button
						>
					{/if}
				{:else}
					<span class="text-2xs text-hint">None</span>
				{/if}
			</div>
		{/each}
		<div class="flex justify-end">
			<Button variant="subtle" unifiedSize="xs" onclick={() => actions.onManage(section)}>
				Edit
			</Button>
		</div>
	{/if}
</div>
