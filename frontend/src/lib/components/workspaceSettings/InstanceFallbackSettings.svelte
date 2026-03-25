<script lang="ts">
	import { type AIProvider, type InstanceAISummary } from '$lib/gen'
	import { AI_PROVIDERS } from '../copilot/lib'
	import Badge from '../common/badge/Badge.svelte'
	import Button from '../common/button/Button.svelte'
	import SettingCard from '../instanceSettings/SettingCard.svelte'

	interface Props {
		instanceAiSummary?: InstanceAISummary
		showWorkspaceOverrideEditor?: boolean
		onToggleOverride: () => void
	}

	let {
		instanceAiSummary = undefined,
		showWorkspaceOverrideEditor = false,
		onToggleOverride
	}: Props = $props()

	let sortedInstanceProviders = $derived(
		[...(instanceAiSummary?.providers ?? [])].sort((left, right) =>
			left.provider.localeCompare(right.provider)
		)
	)

	function getProviderLabel(provider: AIProvider): string {
		return AI_PROVIDERS[provider]?.label ?? provider
	}
</script>

{#if instanceAiSummary}
	<SettingCard label="Active instance AI">
		<div class="flex flex-col gap-4 p-4 rounded-md border bg-surface-tertiary">
			<p class="text-xs text-secondary">
				This workspace is currently using the instance AI defaults shown below.
			</p>

			<div class="flex flex-col gap-3">
				{#each sortedInstanceProviders as providerSummary}
					<div class="rounded-md border bg-surface p-3 flex flex-col gap-2">
						<div class="flex items-center gap-2">
							<span class="text-xs font-medium">
								{getProviderLabel(providerSummary.provider)}
							</span>
							<Badge color="blue">Instance</Badge>
						</div>
						<div class="flex flex-wrap gap-1">
							{#each providerSummary.models as model}
								<Badge color="gray">{model}</Badge>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			{#if instanceAiSummary.default_model}
				<div class="text-xs text-secondary">
					Default chat model:
					<span class="text-primary font-medium">{instanceAiSummary.default_model.model}</span>
					<span class="text-tertiary">
						({getProviderLabel(instanceAiSummary.default_model.provider)})
					</span>
				</div>
			{/if}

			{#if instanceAiSummary.code_completion_model}
				<div class="text-xs text-secondary">
					Code completion model:
					<span class="text-primary font-medium">
						{instanceAiSummary.code_completion_model.model}
					</span>
					<span class="text-tertiary">
						({getProviderLabel(instanceAiSummary.code_completion_model.provider)})
					</span>
				</div>
			{/if}
		</div>
	</SettingCard>
{/if}

<SettingCard label="Workspace override">
	<div class="flex flex-col gap-3 p-4 rounded-md border bg-surface-tertiary">
		<p class="text-xs text-secondary">
			Create workspace-specific AI settings only if this workspace needs to override the active
			instance defaults.
		</p>
		<div>
			<Button onclick={onToggleOverride} variant="default" unifiedSize="sm">
				{showWorkspaceOverrideEditor ? 'Hide override form' : 'Override for this workspace'}
			</Button>
		</div>
	</div>
</SettingCard>
