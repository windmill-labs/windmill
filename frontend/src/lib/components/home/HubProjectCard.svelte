<script lang="ts">
	import { Download, LayoutGrid, Star } from 'lucide-svelte'
	import { appIconComponent } from '$lib/components/icons'
	import { Button } from '$lib/components/common'
	import {
		formatItemCounts,
		hubProjectLogoUrl,
		hubProjectUrl,
		type HubProject
	} from './hubProjects'

	interface Props {
		project: HubProject
		onImport?: (project: HubProject) => void
	}

	let { project, onImport }: Props = $props()

	// hasLogo is computed when the Hub serves the summary; if the logo is deleted after
	// that, fall through to the integration icons rather than a broken-image glyph.
	let logoBroken = $state(false)

	let icons = $derived(
		project.apps
			.map((app) => ({ app, component: appIconComponent(app) }))
			.filter((i) => i.component)
	)
	let shownIcons = $derived(icons.slice(0, 4))
	let overflow = $derived(icons.length - shownIcons.length)
	let counts = $derived(formatItemCounts(project.counts))
</script>

<div
	class="group flex flex-col rounded-lg border border-light bg-surface-tertiary overflow-hidden
	       transition-colors hover:border-selected"
>
	<a
		href={hubProjectUrl(project.slug)}
		target="_blank"
		rel="noreferrer"
		title="View {project.name} on Windmill Hub"
		class="h-24 flex items-center justify-center px-6 bg-surface-secondary"
	>
		{#if project.hasLogo && !logoBroken}
			<img
				src={hubProjectLogoUrl(project.slug)}
				alt=""
				class="max-h-12 max-w-32 object-contain"
				loading="lazy"
				onerror={() => (logoBroken = true)}
			/>
		{:else if shownIcons.length === 0}
			<LayoutGrid size={28} class="text-hint" />
		{:else}
			<div class="flex items-center gap-3">
				{#each shownIcons as icon (icon.app)}
					{@const Icon = icon.component}
					<Icon
						width={shownIcons.length === 1 ? '36px' : '28px'}
						height={shownIcons.length === 1 ? '36px' : '28px'}
					/>
				{/each}
				{#if overflow > 0}
					<span class="text-xs font-semibold text-secondary">+{overflow}</span>
				{/if}
			</div>
		{/if}
	</a>

	<div class="flex flex-col gap-1 p-3 grow">
		<div class="flex items-start justify-between gap-2">
			<h3 class="text-sm font-medium text-emphasis leading-snug line-clamp-1">{project.name}</h3>
			{#if project.stars > 0}
				<span class="flex items-center gap-1 text-2xs text-secondary shrink-0 mt-0.5">
					<Star size={12} />{project.stars}
				</span>
			{/if}
		</div>
		<p class="text-xs text-secondary line-clamp-2">{project.summary}</p>

		<div class="flex items-center justify-between gap-2 mt-auto pt-3">
			<span class="text-2xs text-hint truncate" title="{counts} by {project.author}">
				{counts}
			</span>
			<Button
				unifiedSize="xs"
				variant="default"
				startIcon={{ icon: Download }}
				onClick={() => onImport?.(project)}
			>
				Import
			</Button>
		</div>
	</div>
</div>
