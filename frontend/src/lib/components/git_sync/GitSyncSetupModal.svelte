<script lang="ts">
	import type { Snippet } from 'svelte'
	import { untrack } from 'svelte'
	import { ArrowRight, Database } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import EEOnly from '../EEOnly.svelte'
	import GithubIcon from '../icons/GithubIcon.svelte'
	import GitlabIcon from '../icons/GitlabIcon.svelte'
	import { ResourceService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'

	type Provider = 'gitlab' | 'github_app' | 'github_pat' | 'existing'

	type Props = {
		opened: boolean
		title: string
		/** Resources already attached to a repository of this workspace, which cannot be picked twice. */
		usedResourcePaths: string[]
		onExistingResource: (path: string) => void
	}

	let { opened = $bindable(), title, usedResourcePaths, onExistingResource }: Props = $props()

	const STEPS = ['Choose a provider', 'Connect the repository', 'Configure sync']

	let provider: Provider | undefined = $state(undefined)
	let resourcePath: string | undefined = $state(undefined)

	async function reset() {
		provider = undefined
		resourcePath = undefined
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			const resources = await ResourceService.listResource({
				workspace,
				resourceType: 'git_repository'
			})
			const firstUnused = resources.find((r) => !usedResourcePaths.includes(r.path))
			if (provider === undefined && firstUnused) {
				provider = 'existing'
				resourcePath = firstUnused.path
			}
		} catch (e) {
			console.error('Failed to list git_repository resources', e)
		}
	}

	$effect(() => {
		if (opened) untrack(() => reset())
	})

	const canContinue = $derived(provider === 'existing' && !!resourcePath)

	function next() {
		if (provider === 'existing' && resourcePath) {
			opened = false
			onExistingResource(resourcePath)
		}
	}
</script>

<Modal2
	bind:isOpen={opened}
	target="#content"
	formStyling
	{title}
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="lg"
>
	<div class="flex h-full flex-col gap-4">
		<Stepper tabs={STEPS} selectedIndex={0} maxReachedIndex={0} small />

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-2">
				{#snippet gitlabIcon()}
					<GitlabIcon height={18} width={18} />
				{/snippet}
				{@render providerCard(
					'gitlab',
					gitlabIcon,
					'GitLab',
					'Connect a GitLab repository with an access token.'
				)}
				{#snippet githubIcon()}
					<GithubIcon height={18} width={18} />
				{/snippet}
				{@render providerCard(
					'github_app',
					githubIcon,
					'GitHub App',
					'Install the Windmill GitHub App on your repository. Enables webhooks, pull requests and commit checks.',
					!$enterpriseLicense
				)}
				{@render providerCard(
					'github_pat',
					githubIcon,
					'GitHub personal access token',
					'Connect a GitHub repository with a fine-grained personal access token.'
				)}

				{#if provider === 'existing'}
					{#snippet resourceIcon()}
						<Database size={18} class="text-secondary" />
					{/snippet}
					{@render providerCard(
						'existing',
						resourceIcon,
						'Use an existing git_repository resource',
						'Pick a resource that already holds the repository URL and its credentials.'
					)}
					<ResourcePicker
						bind:value={resourcePath}
						resourceType="git_repository"
						excludedValues={usedResourcePaths}
					/>
				{:else}
					<div>
						<Button variant="subtle" unifiedSize="xs" onClick={() => (provider = 'existing')}>
							Use an existing resource instead
						</Button>
					</div>
				{/if}
			</div>

			<div class="flex justify-end pt-3">
				<Button
					unifiedSize="sm"
					variant="accent"
					disabled={!canContinue}
					endIcon={{ icon: ArrowRight }}
					onClick={next}
				>
					Next
				</Button>
			</div>
		</div>
	</div>
</Modal2>

{#snippet providerCard(
	key: Provider,
	icon: Snippet,
	title: string,
	subtitle: string,
	eeOnly: boolean = false
)}
	{@const selected = provider === key}
	<button
		class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
			? 'border-border-selected/50 bg-surface-accent-selected'
			: 'border-border-light hover:bg-surface-hover'} disabled:opacity-60 disabled:cursor-not-allowed"
		disabled={eeOnly}
		onclick={() => (provider = key)}
	>
		<span class="mt-0.5 shrink-0">{@render icon()}</span>
		<span class="flex flex-col gap-0.5 min-w-0">
			<span
				class="text-xs font-medium flex items-center gap-2 {selected
					? 'text-accent'
					: 'text-emphasis'}"
			>
				{title}
				{#if eeOnly}<EEOnly />{/if}
			</span>
			<span class="text-xs text-secondary font-normal">{subtitle}</span>
		</span>
	</button>
{/snippet}
