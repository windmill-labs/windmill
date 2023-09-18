<script lang="ts">
	import { ExternalLink } from "lucide-svelte"
	import PanelSection from "./common/PanelSection.svelte"
	import type { components } from "../component"
	import { getComponentControl } from '../componentsPanel/componentControlUtils'
	import { Highlight } from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import { slide } from "svelte/transition"
	import { Button } from "$lib/components/common"
  export let type: keyof typeof components

  const componentControls = getComponentControl(type)

  let collapsed: boolean = true

</script>

{#if componentControls?.length > 0 }
  <PanelSection title="Controls" >

    <div slot="action" class="flex justify-end flex-wrap gap-1">
      <Button
        color="light"
        size="xs"
        variant="border"
        on:click={() => {
          collapsed = !collapsed
        
        }}
      >
        {collapsed ? "Show" : "Hide"} details
      </Button>
    </div>

    {#if !collapsed}
      <div transition:slide|local class="text-xs">	This component can be controlled by frontend scripts using these functions:</div>
    {/if}
    {#each componentControls as control}
      <div class="text-xs leading-6 font-semibold">
        {control.title}
      </div>
      {#if !collapsed}
        <div class="text-xs" transition:slide|local>
          {control.description}
        </div>
      {/if}
      <div class="p-1 border w-full">
        <Highlight language={typescript} code={control.example} />
      </div>
      {#if !collapsed}
        <a
          href={control.documentation}
          target="_blank"
          class="text-frost-500 dark:text-frost-300 font-semibold text-xs"
          transition:slide|local
        >
          <div class="flex flex-row gap-2">
            See documentation
            <ExternalLink size="16" />
          </div>
        </a>
      {/if}
    {/each}  
  </PanelSection>
{/if}