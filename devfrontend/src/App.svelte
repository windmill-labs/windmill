<script lang="ts">
  import TestJobLoader from 'windmill-components/components/TestJobLoader.svelte'
  import Kbd from 'windmill-components/components/common/kbd/Kbd.svelte'
  import LogPanel from 'windmill-components/components/scriptEditor/LogPanel.svelte'
  import WindmillIcon from 'windmill-components/components/icons/WindmillIcon.svelte'
  import SchemaForm from 'windmill-components/components/SchemaForm.svelte'
  import Button from 'windmill-components/components/common/button/Button.svelte'
  import { emptySchema, getModifierKey } from 'windmill-components/utils'
  import { inferArgs } from 'windmill-components/infer'
	import github from 'svelte-highlight/styles/github'

  import { Pane, Splitpanes} from 'svelte-splitpanes'
  import { faPlay } from '@fortawesome/free-solid-svg-icons'
  import { CompletedJob, Job, JobService } from 'windmill-client'
  import { workspaceStore } from 'windmill-components/stores';

  let testJobLoader: TestJobLoader

  // Test args input
  let args: Record<string, any> = {}
  let isValid: boolean = true

  // Test
  let testIsLoading = false
  let testJob: Job | undefined
  let pastPreviews: CompletedJob[] = []
  let validCode = true

  type LastEdit = { content: string; path: string; language: string, workspace: string, username: string};

  let currentScript: LastEdit | undefined = undefined

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
    $workspaceStore = currentScript.workspace
    //@ts-ignore
		testJobLoader.runPreview(currentScript.path, currentScript.content, currentScript.language, args, undefined)
	}

  async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: currentScript.workspace,
			jobKinds: 'preview',
			createdBy: currentScript.username,
			scriptPathExact: currentScript.path,
		})
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

  let lastPath = undefined
  async function replaceScript(LastEdit: LastEdit) {
    currentScript = LastEdit
    if (lastPath !== LastEdit.path) {
      schema = emptySchema()
    }
    try {
    //@ts-ignore
    await inferArgs(LastEdit.language, LastEdit.content, schema )
    schema = schema
    lastPath = LastEdit.path
    validCode = true
    } catch (e) {
      console.error(e)
      validCode = false
    }
  }
  
</script>

<svelte:window on:keydown={onKeyDown} />

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>


<svelte:head>
	{@html github}
</svelte:head>

<main class="h-screen w-full">
  <div class="flex flex-col h-full">
    <div class="text-center w-full text-lg truncate py-1">{currentScript?.path ?? 'Not editing a script'} {currentScript?.language ?? ''}</div>
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
          disabled={currentScript === undefined}
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
        <LogPanel lang={currentScript?.language} previewJob={testJob} {pastPreviews} previewIsLoading={testIsLoading} />
      </Pane>
    </Splitpanes>
  </div>
</main>

