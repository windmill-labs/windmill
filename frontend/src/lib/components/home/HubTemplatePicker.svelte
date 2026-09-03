<script lang="ts">
	import { ArrowRight, ArrowUpRight, LayoutGrid, Star } from 'lucide-svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import {
		hubAppIcon,
		hubBrowserUrl,
		hubProjectCatalogue,
		type HubProjectPick
	} from '$lib/hubProject'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		onPick: (project: HubProjectPick) => void
	}

	let { onPick }: Props = $props()

	let list: InfiniteList | undefined = $state(undefined)

	// The hub serves its whole catalogue in one response, so paging happens here: the list
	// asks for a window and gets a slice of what `hubProjectCatalogue` already holds. Should
	// the hub ever paginate, only this loader changes.
	$effect(() => {
		const workspace = $workspaceStore
		if (!list || !workspace) return
		list.setLoader(async (page: number, perPage: number) => {
			const all = await hubProjectCatalogue(workspace)
			return all.slice((page - 1) * perPage, page * perPage)
		})
		list.loadData('refresh')
	})

	let hubUrl = $state('https://hub.windmill.dev')
	void hubBrowserUrl()
		.then((u) => (hubUrl = u))
		.catch(() => {})
	// `hubBrowserUrl` hands back the instance setting as the admin wrote it, so a scheme-less
	// one would make `new URL` throw — in render, which takes the popover down with it.
	let hubHost = $derived.by(() => {
		try {
			return new URL(hubUrl).host
		} catch {
			return hubUrl
		}
	})
</script>

<!-- The popover gives this box a definite height; the list takes what the header leaves and
     scrolls inside it, which is also what lets it page. -->
<div class="flex min-h-0 w-[380px] flex-col">
	<!-- The hub is named once, as the link to it: a footer row saying the same thing again is
	     a second line spent on somewhere the reader is not going. -->
	<p class="px-3 pb-2 pt-3 text-[11.5px] leading-snug text-hint">
		Working projects from
		<a
			href="{hubUrl}/projects"
			target="_blank"
			rel="noreferrer"
			class="inline-flex items-baseline gap-0.5 text-secondary hover:text-emphasis hover:underline"
		>
			{hubHost}<ArrowUpRight size={11} class="self-center" />
		</a>
		— imported as a folder in this workspace.
	</p>

	<div class="min-h-0 flex-1 border-t border-border-light">
		<InfiniteList bind:this={list} noBorder rounded={false} containerClass="h-full">
			{#snippet customRow({ item }: { item: HubProjectPick })}
				{@const Icon = hubAppIcon(item.iconApps[0] ?? '')}
				<tr>
					<td class="p-0">
						<button
							class="group flex w-full items-start gap-3 border-b border-border-light px-3 py-2.5 text-left hover:bg-surface-hover"
							onclick={() => onPick(item)}
						>
							<div class="flex size-[22px] shrink-0 items-center justify-center">
								{#if item.logoUrl}
									<img src={item.logoUrl} alt="" class="max-h-[22px] max-w-[22px] object-contain" />
								{:else if Icon}
									<Icon size={20} />
								{:else}
									<LayoutGrid size={18} class="text-tertiary" />
								{/if}
							</div>

							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-1.5">
									<span class="truncate text-xs font-semibold text-emphasis">{item.name}</span>
									{#if item.stars > 0}
										<span
											class="flex shrink-0 items-center gap-0.5 text-2xs font-normal text-tertiary"
										>
											<Star size={11} />{item.stars}
										</span>
									{/if}
									<!-- The arrow is the only thing that appears on hover: the whole row is the
									     control, so it says where the row goes rather than acting as a button. -->
									<ArrowRight
										size={13}
										class="shrink-0 text-tertiary opacity-0 transition group-hover:opacity-100"
									/>
								</div>
								<!-- What the project is, in its own words. The item counts it used to carry say
								     nothing about whether this is the project you want; the wizard shows them on
								     the step where they matter, right before the import runs. -->
								<p
									class="mt-1 line-clamp-4 text-[11.5px] font-normal leading-relaxed text-secondary"
								>
									{item.description || item.summary}
								</p>
							</div>
						</button>
					</td>
				</tr>
			{/snippet}

			{#snippet empty()}
				<p class="px-3 py-6 text-xs text-secondary">
					Could not reach the hub. You can still browse its projects in a new tab.
				</p>
			{/snippet}
		</InfiniteList>
	</div>
</div>
