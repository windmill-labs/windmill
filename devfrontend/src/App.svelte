<script lang="ts">
  import TestJobLoader from 'windmill-components/components/TestJobLoader.svelte'
  import Kbd from 'windmill-components/components/common/kbd/Kbd.svelte'
  import LogPanel from 'windmill-components/components/scriptEditor/LogPanel.svelte'
  import WindmillIcon from 'windmill-components/components/icons/WindmillIcon.svelte'
  import SchemaForm from 'windmill-components/components/SchemaForm.svelte'
  import Button from 'windmill-components/components/common/button/Button.svelte'
  import { emptySchema, getModifierKey } from 'windmill-components/utils'
  import { inferArgs } from 'windmill-components/infer'

  import type { CompletedJob, Job, Preview } from 'windmill-components/package/gen';
  import { Pane, Splitpanes} from 'svelte-splitpanes'
  import { faPlay } from '@fortawesome/free-solid-svg-icons'

  let testJobLoader: TestJobLoader

  // Test args input
  let args: Record<string, any> = {}
  let isValid: boolean = true

  // Test
  let testIsLoading = false
  let testJob: Job | undefined
  let pastPreviews: CompletedJob[] = []
  let lang = 'deno' as Preview.language
  let validCode = true


  let currentScript: {path: string, content: string} | undefined = undefined

  let schema = emptySchema()
  const href = window.location.href;
  const indexQ = href.indexOf('?')
  const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined;

  const port = searchParams?.get('port') || '3000';
  const socket = new WebSocket(`ws://localhost:${port}/ws`);

  // Connection opened
  socket.addEventListener("open", (event) => {
    socket.send("Hello Server!");
  });

  // Listen for messages
  socket.addEventListener("message", (event) => {
    let data: any | undefined = undefined;
    try {
      data = JSON.parse(event.data);
    } catch {
      console.log("Received invalid JSON: " + event.data);
      return
    }
    replaceScript(data);
  });

  function runTest() {
		testJobLoader.runPreview(path, code, lang, args,)
	}

  async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}



  let lastPath = undefined
  async function replaceScript({path, content, language}: {path: string, content: string, language: "deno" | "python3" | "go" | "bash"}) {
    currentScript = {path, content}
    if (lastPath !== path) {
      schema = emptySchema()
    }
    try {
    await inferArgs(language, content, schema )
    schema = schema
    lastPath = path
    validCode = true
    } catch (e) {
      console.error(e)
      validCode = false
    }
  }
  
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>


<main class="h-screen w-full">
  <div class="flex flex-col h-full">
    <div class="text-center w-full text-lg truncate py-1">{currentScript?.path ?? 'Not editing a script'}</div>
    {#if !validCode}
      <div class="text-center w-full text-lg truncate py-1 text-red-500">Invalid code</div>
    {/if}
   <div class="flex justify-center pt-1">
      {#if testIsLoading}
        <Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
          <WindmillIcon
            white={true}
            class="mr-2 text-white"
            height="20px"
            width="20px"
            spin="fast"
          />
          Cancel
        </Button>
      {:else}
        <Button
          color="dark"
          on:click={() => {
            runTest()
          }}
          btnClasses="w-full"
          size="xs"
          startIcon={{
            icon: faPlay,
            classes: 'animate-none'
          }}
        >
          {#if testIsLoading}
            Running
          {:else}
            Test&nbsp;<Kbd small>{getModifierKey()}</Kbd>
            <Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
          {/if}
        </Button>
      {/if}
    </div>
    <Splitpanes horizontal class="h-full">
      <Pane size={33}>
        <div class="px-2">
          <div class="break-words relative font-sans">
            <SchemaForm compact {schema} bind:args bind:isValid />
          </div>
        </div>
      </Pane>
      <Pane size={67}>
        <LogPanel {lang} previewJob={testJob} {pastPreviews} previewIsLoading={testIsLoading} />
      </Pane>
    </Splitpanes>
  </div>
</main>

