// The starter app is a tour of what a raw app can do: each card calls one of the
// STARTER_RUNNABLES below through a different `./wmill` export. The three framework
// variants share `indexCss` and must stay in step with each other and with the runnables.

const reactIndex = `
import React from 'react'

import { createRoot } from 'react-dom/client'
import App from './App'

const root = createRoot(document.getElementById('root')!);
root.render(<App/>);
`

const STATS_SAMPLE_TEXT =
  'Windmill turns scripts into apps. Write the frontend in React, Svelte or Vue, and the backend in TypeScript, Python or any other supported language. Scripts run on workers, and the app calls them like functions.'

const appTsx = `import React, { useState } from 'react'
import { backend, backendAsync, getJob, streamJob } from './wmill'
import './index.css'

// Windmill injects the viewer (absent for anonymous visitors) and the workspace.
const ctx = (window as any).ctx ?? {}
const viewer: { name?: string; username?: string } | undefined = ctx.ctx
const firstName = (viewer?.name || viewer?.username || '').split(' ')[0]

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

function errorMessage(e: any): string {
  return e?.message ?? (typeof e === 'string' ? e : JSON.stringify(e))
}

function Card(props: {
  step: string
  title: string
  description: string
  code: string
  children: React.ReactNode
}) {
  return (
    <section className="card">
      <div className="card-header">
        <span className="card-step">{props.step}</span>
        <div>
          <h2>{props.title}</h2>
          <p className="muted">{props.description}</p>
        </div>
      </div>
      <div className="card-body">{props.children}</div>
      <pre className="snippet">
        <code>{props.code}</code>
      </pre>
    </section>
  )
}

function GreetDemo() {
  const [name, setName] = useState(firstName || 'world')
  const [result, setResult] = useState<{ message: string; ran_at: string }>()
  const [error, setError] = useState<string>()
  const [loading, setLoading] = useState(false)

  async function run() {
    setLoading(true)
    setError(undefined)
    try {
      setResult(await backend.greet({ name }))
    } catch (e) {
      setError(errorMessage(e))
    }
    setLoading(false)
  }

  return (
    <Card
      step="1"
      title="Call a backend runnable"
      description="Runnables are scripts that run on Windmill workers. Call them like async functions; context fields add who is calling."
      code="const res = await backend.greet({ name })"
    >
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault()
          run()
        }}
      >
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Your name" />
        <button type="submit" disabled={loading}>
          {loading ? 'Running...' : 'Run'}
        </button>
      </form>
      <div className="output">
        {error ? (
          <span className="error">{error}</span>
        ) : result ? (
          <>
            <strong>{result.message}</strong>
            <span className="muted"> Ran at {new Date(result.ran_at).toLocaleTimeString()}</span>
          </>
        ) : (
          <span className="muted">The result shows up here.</span>
        )}
      </div>
    </Card>
  )
}

type Stats = { words: number; characters: number; sentences: number; top_words: string[] }

function StatsDemo() {
  const [text, setText] = useState(${JSON.stringify(STATS_SAMPLE_TEXT)})
  const [stats, setStats] = useState<Stats>()
  const [error, setError] = useState<string>()
  const [loading, setLoading] = useState(false)

  async function run() {
    setLoading(true)
    setError(undefined)
    try {
      setStats(await backend.word_stats({ text }))
    } catch (e) {
      setError(errorMessage(e))
    }
    setLoading(false)
  }

  return (
    <Card
      step="2"
      title="Use any language"
      description="Each runnable picks its own language. This one is Python, called the same way."
      code="const stats = await backend.word_stats({ text })"
    >
      <textarea rows={4} value={text} onChange={(e) => setText(e.target.value)} />
      <div className="row">
        <button onClick={run} disabled={loading}>
          {loading ? 'Analyzing...' : 'Analyze with Python'}
        </button>
      </div>
      {error && <div className="output error">{error}</div>}
      {stats && (
        <div className="stats">
          <div><strong>{stats.words}</strong><span className="muted">words</span></div>
          <div><strong>{stats.sentences}</strong><span className="muted">sentences</span></div>
          <div><strong>{stats.characters}</strong><span className="muted">characters</span></div>
          <div className="tags">
            {stats.top_words.map((w) => <span key={w} className="tag">{w}</span>)}
          </div>
        </div>
      )}
    </Card>
  )
}

type JobRow = { id: string; seconds: number; status: 'queued' | 'running' | 'success' | 'failure' }

function JobsDemo() {
  const [jobs, setJobs] = useState<JobRow[]>([])
  const [error, setError] = useState<string>()

  const update = (id: string, status: JobRow['status']) =>
    setJobs((rows) => rows.map((r) => (r.id === id ? { ...r, status } : r)))

  async function start() {
    setError(undefined)
    try {
      const seconds = 2 + Math.floor(Math.random() * 4)
      const id = await backendAsync.slow_task({ seconds })
      setJobs((rows) => [{ id, seconds, status: 'queued' as const }, ...rows].slice(0, 4))
      while (true) {
        await sleep(500)
        const job = await getJob(id)
        if (job.type === 'CompletedJob') {
          update(id, job.success ? 'success' : 'failure')
          return
        }
        update(id, job.started_at ? 'running' : 'queued')
      }
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  return (
    <Card
      step="3"
      title="Run jobs in the background"
      description="Start long jobs without blocking the UI, then poll their status. Start several at once."
      code={'const id = await backendAsync.slow_task({ seconds })\\nconst job = await getJob(id)'}
    >
      <div className="row">
        <button onClick={start}>Start a job</button>
      </div>
      {error && <div className="output error">{error}</div>}
      <ul className="jobs">
        {jobs.length === 0 && <li className="muted">No jobs yet.</li>}
        {jobs.map((job) => (
          <li key={job.id}>
            <span className={'pill ' + job.status}>{job.status}</span>
            <code title={job.id}>{job.id.slice(-8)}</code>
            <span className="muted">{job.seconds}s</span>
            <div className="bar">
              <div className={'bar-fill ' + job.status} style={{ animationDuration: job.seconds + 's' }} />
            </div>
          </li>
        ))}
      </ul>
    </Card>
  )
}

function StreamDemo() {
  const [text, setText] = useState('')
  const [error, setError] = useState<string>()
  const [streaming, setStreaming] = useState(false)

  async function run() {
    setText('')
    setError(undefined)
    setStreaming(true)
    try {
      const id = await backendAsync.stream_text({})
      await streamJob(id, (update) => {
        if (update.new_result_stream) setText((t) => t + update.new_result_stream)
      })
    } catch (e) {
      setError(errorMessage(e))
    }
    setStreaming(false)
  }

  return (
    <Card
      step="4"
      title="Stream results"
      description="A runnable can yield its output bit by bit. Render it as it arrives, like an LLM answer."
      code="await streamJob(id, (u) => append(u.new_result_stream))"
    >
      <div className="row">
        <button onClick={run} disabled={streaming}>
          {streaming ? 'Streaming...' : 'Start streaming'}
        </button>
      </div>
      <div className="output stream">
        {error ? (
          <span className="error">{error}</span>
        ) : text || streaming ? (
          <>
            {text}
            {streaming && <span className="caret" />}
          </>
        ) : (
          <span className="muted">Streamed text shows up here.</span>
        )}
      </div>
    </Card>
  )
}

const App = () => {
  return (
    <main className="app">
      <header className="hero">
        <span className="eyebrow">Windmill app{ctx.workspace ? ' · ' + ctx.workspace : ''}</span>
        <h1>{firstName ? 'Welcome, ' + firstName : 'Welcome'}</h1>
        <p className="muted">
          A React frontend wired to backend runnables. Each card shows one thing an app can do.
          Edit App.tsx and the runnables to make it yours, or ask the AI to build on it.
        </p>
      </header>
      <div className="grid">
        <GreetDemo />
        <StatsDemo />
        <JobsDemo />
        <StreamDemo />
      </div>
    </main>
  )
}

export default App
`

const appSvelte = `<script lang="ts">
  import { backend, backendAsync, getJob, streamJob } from './wmill'

  // Windmill injects the viewer (absent for anonymous visitors) and the workspace.
  const ctx = (window as any).ctx ?? {}
  const viewer: { name?: string; username?: string } | undefined = ctx.ctx
  const firstName = (viewer?.name || viewer?.username || '').split(' ')[0]

  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

  function errorMessage(e: any): string {
    return e?.message ?? (typeof e === 'string' ? e : JSON.stringify(e))
  }

  // 1. Call a backend runnable
  let name = $state(firstName || 'world')
  let greeting = $state<{ message: string; ran_at: string }>()
  let greetError = $state<string>()
  let greeting_loading = $state(false)

  async function greet(e: SubmitEvent) {
    e.preventDefault()
    greeting_loading = true
    greetError = undefined
    try {
      greeting = await backend.greet({ name })
    } catch (e) {
      greetError = errorMessage(e)
    }
    greeting_loading = false
  }

  // 2. Use any language
  let text = $state(${JSON.stringify(STATS_SAMPLE_TEXT)})
  let stats = $state<{ words: number; characters: number; sentences: number; top_words: string[] }>()
  let statsError = $state<string>()
  let statsLoading = $state(false)

  async function analyze() {
    statsLoading = true
    statsError = undefined
    try {
      stats = await backend.word_stats({ text })
    } catch (e) {
      statsError = errorMessage(e)
    }
    statsLoading = false
  }

  // 3. Run jobs in the background
  type JobRow = { id: string; seconds: number; status: 'queued' | 'running' | 'success' | 'failure' }
  let jobs = $state<JobRow[]>([])
  let jobsError = $state<string>()

  function setStatus(id: string, status: JobRow['status']) {
    const row = jobs.find((r) => r.id === id)
    if (row) row.status = status
  }

  async function startJob() {
    jobsError = undefined
    try {
      const seconds = 2 + Math.floor(Math.random() * 4)
      const id = await backendAsync.slow_task({ seconds })
      jobs = [{ id, seconds, status: 'queued' as const }, ...jobs].slice(0, 4)
      while (true) {
        await sleep(500)
        const job = await getJob(id)
        if (job.type === 'CompletedJob') {
          setStatus(id, job.success ? 'success' : 'failure')
          return
        }
        setStatus(id, job.started_at ? 'running' : 'queued')
      }
    } catch (e) {
      jobsError = errorMessage(e)
    }
  }

  // 4. Stream results
  let streamed = $state('')
  let streamError = $state<string>()
  let streaming = $state(false)

  async function stream() {
    streamed = ''
    streamError = undefined
    streaming = true
    try {
      const id = await backendAsync.stream_text({})
      await streamJob(id, (update) => {
        if (update.new_result_stream) streamed += update.new_result_stream
      })
    } catch (e) {
      streamError = errorMessage(e)
    }
    streaming = false
  }
</script>

{#snippet header(step: string, title: string, description: string)}
  <div class="card-header">
    <span class="card-step">{step}</span>
    <div>
      <h2>{title}</h2>
      <p class="muted">{description}</p>
    </div>
  </div>
{/snippet}

<main class="app">
  <header class="hero">
    <span class="eyebrow">Windmill app{ctx.workspace ? ' · ' + ctx.workspace : ''}</span>
    <h1>{firstName ? 'Welcome, ' + firstName : 'Welcome'}</h1>
    <p class="muted">
      A Svelte frontend wired to backend runnables. Each card shows one thing an app can do.
      Edit App.svelte and the runnables to make it yours, or ask the AI to build on it.
    </p>
  </header>

  <div class="grid">
    <section class="card">
      {@render header('1', 'Call a backend runnable', 'Runnables are scripts that run on Windmill workers. Call them like async functions; context fields add who is calling.')}
      <div class="card-body">
        <form class="row" onsubmit={greet}>
          <input bind:value={name} placeholder="Your name" />
          <button type="submit" disabled={greeting_loading}>
            {greeting_loading ? 'Running...' : 'Run'}
          </button>
        </form>
        <div class="output">
          {#if greetError}
            <span class="error">{greetError}</span>
          {:else if greeting}
            <strong>{greeting.message}</strong>
            <span class="muted"> Ran at {new Date(greeting.ran_at).toLocaleTimeString()}</span>
          {:else}
            <span class="muted">The result shows up here.</span>
          {/if}
        </div>
      </div>
      <pre class="snippet"><code>const res = await backend.greet(&#123; name &#125;)</code></pre>
    </section>

    <section class="card">
      {@render header('2', 'Use any language', 'Each runnable picks its own language. This one is Python, called the same way.')}
      <div class="card-body">
        <textarea rows="4" bind:value={text}></textarea>
        <div class="row">
          <button onclick={analyze} disabled={statsLoading}>
            {statsLoading ? 'Analyzing...' : 'Analyze with Python'}
          </button>
        </div>
        {#if statsError}
          <div class="output error">{statsError}</div>
        {/if}
        {#if stats}
          <div class="stats">
            <div><strong>{stats.words}</strong><span class="muted">words</span></div>
            <div><strong>{stats.sentences}</strong><span class="muted">sentences</span></div>
            <div><strong>{stats.characters}</strong><span class="muted">characters</span></div>
            <div class="tags">
              {#each stats.top_words as word (word)}
                <span class="tag">{word}</span>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      <pre class="snippet"><code>const stats = await backend.word_stats(&#123; text &#125;)</code></pre>
    </section>

    <section class="card">
      {@render header('3', 'Run jobs in the background', 'Start long jobs without blocking the UI, then poll their status. Start several at once.')}
      <div class="card-body">
        <div class="row">
          <button onclick={startJob}>Start a job</button>
        </div>
        {#if jobsError}
          <div class="output error">{jobsError}</div>
        {/if}
        <ul class="jobs">
          {#each jobs as job (job.id)}
            <li>
              <span class="pill {job.status}">{job.status}</span>
              <code title={job.id}>{job.id.slice(-8)}</code>
              <span class="muted">{job.seconds}s</span>
              <div class="bar">
                <div class="bar-fill {job.status}" style:animation-duration="{job.seconds}s"></div>
              </div>
            </li>
          {:else}
            <li class="muted">No jobs yet.</li>
          {/each}
        </ul>
      </div>
      <pre class="snippet"><code>const id = await backendAsync.slow_task(&#123; seconds &#125;)
const job = await getJob(id)</code></pre>
    </section>

    <section class="card">
      {@render header('4', 'Stream results', 'A runnable can yield its output bit by bit. Render it as it arrives, like an LLM answer.')}
      <div class="card-body">
        <div class="row">
          <button onclick={stream} disabled={streaming}>
            {streaming ? 'Streaming...' : 'Start streaming'}
          </button>
        </div>
        <div class="output stream">
          {#if streamError}
            <span class="error">{streamError}</span>
          {:else if streamed || streaming}
            {streamed}{#if streaming}<span class="caret"></span>{/if}
          {:else}
            <span class="muted">Streamed text shows up here.</span>
          {/if}
        </div>
      </div>
      <pre class="snippet"><code>await streamJob(id, (u) => append(u.new_result_stream))</code></pre>
    </section>
  </div>
</main>
`

const indexSvelte = `
import { mount } from 'svelte';
import App from './App.svelte'
import './index.css'

const app = mount(App, { target: document.getElementById("root")! });

export default app;
`

const appVue = `<template>
  <main class="app">
    <header class="hero">
      <span class="eyebrow">Windmill app{{ ctx.workspace ? ' · ' + ctx.workspace : '' }}</span>
      <h1>{{ firstName ? 'Welcome, ' + firstName : 'Welcome' }}</h1>
      <p class="muted">
        A Vue frontend wired to backend runnables. Each card shows one thing an app can do.
        Edit App.vue and the runnables to make it yours, or ask the AI to build on it.
      </p>
    </header>

    <div class="grid">
      <section class="card">
        <div class="card-header">
          <span class="card-step">1</span>
          <div>
            <h2>Call a backend runnable</h2>
            <p class="muted">Runnables are scripts that run on Windmill workers. Call them like async functions; context fields add who is calling.</p>
          </div>
        </div>
        <div class="card-body">
          <form class="row" @submit.prevent="greet">
            <input v-model="name" placeholder="Your name" />
            <button type="submit" :disabled="greetLoading">{{ greetLoading ? 'Running...' : 'Run' }}</button>
          </form>
          <div class="output">
            <span v-if="greetError" class="error">{{ greetError }}</span>
            <template v-else-if="greeting">
              <strong>{{ greeting.message }}</strong>
              <span class="muted"> Ran at {{ new Date(greeting.ran_at).toLocaleTimeString() }}</span>
            </template>
            <span v-else class="muted">The result shows up here.</span>
          </div>
        </div>
        <pre class="snippet"><code>const res = await backend.greet({ name })</code></pre>
      </section>

      <section class="card">
        <div class="card-header">
          <span class="card-step">2</span>
          <div>
            <h2>Use any language</h2>
            <p class="muted">Each runnable picks its own language. This one is Python, called the same way.</p>
          </div>
        </div>
        <div class="card-body">
          <textarea rows="4" v-model="text"></textarea>
          <div class="row">
            <button @click="analyze" :disabled="statsLoading">
              {{ statsLoading ? 'Analyzing...' : 'Analyze with Python' }}
            </button>
          </div>
          <div v-if="statsError" class="output error">{{ statsError }}</div>
          <div v-if="stats" class="stats">
            <div><strong>{{ stats.words }}</strong><span class="muted">words</span></div>
            <div><strong>{{ stats.sentences }}</strong><span class="muted">sentences</span></div>
            <div><strong>{{ stats.characters }}</strong><span class="muted">characters</span></div>
            <div class="tags">
              <span v-for="word in stats.top_words" :key="word" class="tag">{{ word }}</span>
            </div>
          </div>
        </div>
        <pre class="snippet"><code>const stats = await backend.word_stats({ text })</code></pre>
      </section>

      <section class="card">
        <div class="card-header">
          <span class="card-step">3</span>
          <div>
            <h2>Run jobs in the background</h2>
            <p class="muted">Start long jobs without blocking the UI, then poll their status. Start several at once.</p>
          </div>
        </div>
        <div class="card-body">
          <div class="row">
            <button @click="startJob">Start a job</button>
          </div>
          <div v-if="jobsError" class="output error">{{ jobsError }}</div>
          <ul class="jobs">
            <li v-if="jobs.length === 0" class="muted">No jobs yet.</li>
            <li v-for="job in jobs" :key="job.id">
              <span :class="['pill', job.status]">{{ job.status }}</span>
              <code :title="job.id">{{ job.id.slice(-8) }}</code>
              <span class="muted">{{ job.seconds }}s</span>
              <div class="bar">
                <div :class="['bar-fill', job.status]" :style="{ animationDuration: job.seconds + 's' }"></div>
              </div>
            </li>
          </ul>
        </div>
        <pre class="snippet"><code>const id = await backendAsync.slow_task({ seconds })
const job = await getJob(id)</code></pre>
      </section>

      <section class="card">
        <div class="card-header">
          <span class="card-step">4</span>
          <div>
            <h2>Stream results</h2>
            <p class="muted">A runnable can yield its output bit by bit. Render it as it arrives, like an LLM answer.</p>
          </div>
        </div>
        <div class="card-body">
          <div class="row">
            <button @click="stream" :disabled="streaming">{{ streaming ? 'Streaming...' : 'Start streaming' }}</button>
          </div>
          <div class="output stream">
            <span v-if="streamError" class="error">{{ streamError }}</span>
            <template v-else-if="streamed || streaming">
              {{ streamed }}<span v-if="streaming" class="caret"></span>
            </template>
            <span v-else class="muted">Streamed text shows up here.</span>
          </div>
        </div>
        <pre class="snippet"><code>await streamJob(id, (u) => append(u.new_result_stream))</code></pre>
      </section>
    </div>
  </main>
</template>

<script setup>
import { ref } from 'vue'
import { backend, backendAsync, getJob, streamJob } from './wmill'

// Windmill injects the viewer (absent for anonymous visitors) and the workspace.
const ctx = window.ctx ?? {}
const firstName = (ctx.ctx?.name || ctx.ctx?.username || '').split(' ')[0]

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

function errorMessage(e) {
  return e?.message ?? (typeof e === 'string' ? e : JSON.stringify(e))
}

// 1. Call a backend runnable
const name = ref(firstName || 'world')
const greeting = ref()
const greetError = ref()
const greetLoading = ref(false)

async function greet() {
  greetLoading.value = true
  greetError.value = undefined
  try {
    greeting.value = await backend.greet({ name: name.value })
  } catch (e) {
    greetError.value = errorMessage(e)
  }
  greetLoading.value = false
}

// 2. Use any language
const text = ref(${JSON.stringify(STATS_SAMPLE_TEXT)})
const stats = ref()
const statsError = ref()
const statsLoading = ref(false)

async function analyze() {
  statsLoading.value = true
  statsError.value = undefined
  try {
    stats.value = await backend.word_stats({ text: text.value })
  } catch (e) {
    statsError.value = errorMessage(e)
  }
  statsLoading.value = false
}

// 3. Run jobs in the background
const jobs = ref([])
const jobsError = ref()

function setStatus(id, status) {
  const row = jobs.value.find((r) => r.id === id)
  if (row) row.status = status
}

async function startJob() {
  jobsError.value = undefined
  try {
    const seconds = 2 + Math.floor(Math.random() * 4)
    const id = await backendAsync.slow_task({ seconds })
    jobs.value = [{ id, seconds, status: 'queued' }, ...jobs.value].slice(0, 4)
    while (true) {
      await sleep(500)
      const job = await getJob(id)
      if (job.type === 'CompletedJob') {
        setStatus(id, job.success ? 'success' : 'failure')
        return
      }
      setStatus(id, job.started_at ? 'running' : 'queued')
    }
  } catch (e) {
    jobsError.value = errorMessage(e)
  }
}

// 4. Stream results
const streamed = ref('')
const streamError = ref()
const streaming = ref(false)

async function stream() {
  streamed.value = ''
  streamError.value = undefined
  streaming.value = true
  try {
    const id = await backendAsync.stream_text({})
    await streamJob(id, (update) => {
      if (update.new_result_stream) streamed.value += update.new_result_stream
    })
  } catch (e) {
    streamError.value = errorMessage(e)
  }
  streaming.value = false
}
</script>`

const indexVue = `import { createApp } from 'vue'
import App from './App.vue'
import "./index.css";

createApp(App).mount('#root')`

const indexCss = `/* The app renders inside an iframe whose host page has its own theme the
   iframe cannot read, so paint an opaque surface here: without one the host's
   background shows through and dark text lands on it unreadable. */
:root {
  color-scheme: light;
  --bg: #f4f5f8;
  --card: #ffffff;
  --sunken: #efeff4;
  --border: #e2e3eb;
  --fg: #3d4758;
  --fg-strong: #1d2430;
  --muted: #718096;
  --accent: #5074f6;
  --accent-hover: #2c5beb;
  --accent-soft: #e8ebfb;
  --success: #15803d;
  --success-soft: #dcfce7;
  --error: #b91c1c;
  --error-soft: #fef2f2;
  --warning: #a16207;
  --warning-soft: #fef9c3;
}

@media (prefers-color-scheme: dark) {
  :root {
    color-scheme: dark;
    --bg: #242832;
    --card: #2e3441;
    --sunken: #272c35;
    --border: #3b4252;
    --fg: #d4d7dd;
    --fg-strong: #eeeff2;
    --muted: #a9b0ba;
    --accent: #7085db;
    --accent-hover: #5670d5;
    --accent-soft: #343d5c;
    --success: #86efac;
    --success-soft: #1f3a2b;
    --error: #fca5a5;
    --error-soft: #431a1a;
    --warning: #fde68a;
    --warning-soft: #3d3419;
  }
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--fg);
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif;
  font-size: 14px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

h1,
h2 {
  margin: 0;
  color: var(--fg-strong);
}

code,
pre {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.muted {
  color: var(--muted);
}

.error {
  color: var(--error);
}

.app {
  max-width: 1040px;
  margin: 0 auto;
  padding: 40px 20px 56px;
}

.hero {
  margin-bottom: 28px;
}

.hero h1 {
  font-size: 1.75rem;
  font-weight: 650;
  letter-spacing: -0.01em;
  margin: 8px 0 6px;
}

.hero p {
  max-width: 620px;
  margin: 0;
}

.eyebrow {
  display: inline-block;
  padding: 2px 10px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr));
  gap: 16px;
}

.card {
  display: flex;
  flex-direction: column;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 1px 2px rgb(16 24 40 / 0.04);
  overflow: hidden;
}

.card-header {
  display: flex;
  gap: 12px;
  padding: 18px 18px 0;
}

.card-header h2 {
  font-size: 15px;
  font-weight: 600;
}

.card-header p {
  margin: 2px 0 0;
  font-size: 13px;
}

.card-step {
  flex: none;
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 13px;
  font-weight: 700;
}

.card-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 18px 18px;
}

.snippet {
  margin: 0;
  padding: 10px 18px;
  border-top: 1px solid var(--border);
  background: var(--sunken);
  color: var(--muted);
  font-size: 12px;
  white-space: pre-wrap;
}

.row {
  display: flex;
  gap: 8px;
}

input,
textarea {
  flex: 1;
  min-width: 0;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--card);
  color: var(--fg-strong);
  font: inherit;
  resize: vertical;
}

input:focus,
textarea:focus {
  outline: 2px solid var(--accent);
  outline-offset: -1px;
}

button {
  padding: 7px 14px;
  border: none;
  border-radius: 6px;
  background: var(--accent);
  color: #fff;
  font: inherit;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}

button:hover:not(:disabled) {
  background: var(--accent-hover);
}

button:disabled {
  opacity: 0.6;
  cursor: default;
}

.output {
  min-height: 44px;
  padding: 11px 12px;
  border-radius: 6px;
  background: var(--sunken);
  word-break: break-word;
}

.output.error {
  background: var(--error-soft);
}

.output.stream {
  min-height: 96px;
  white-space: pre-wrap;
}

.caret {
  display: inline-block;
  width: 7px;
  height: 1em;
  margin-left: 2px;
  vertical-align: text-bottom;
  background: var(--accent);
  animation: blink 1s steps(1) infinite;
}

@keyframes blink {
  50% {
    opacity: 0;
  }
}

.stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.stats > div {
  display: flex;
  flex-direction: column;
  padding: 10px 12px;
  border-radius: 6px;
  background: var(--sunken);
}

.stats strong {
  font-size: 20px;
  color: var(--fg-strong);
}

.stats span {
  font-size: 12px;
}

.stats .tags {
  grid-column: 1 / -1;
  flex-direction: row;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0;
  background: none;
}

.tag {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}

.jobs {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.jobs li {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--sunken);
  font-size: 13px;
}

.pill {
  min-width: 64px;
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  text-align: center;
}

.pill.queued {
  background: var(--warning-soft);
  color: var(--warning);
}

.pill.running {
  background: var(--accent-soft);
  color: var(--accent);
}

.pill.success {
  background: var(--success-soft);
  color: var(--success);
}

.pill.failure {
  background: var(--error-soft);
  color: var(--error);
}

.bar {
  flex: 1;
  height: 4px;
  border-radius: 999px;
  background: var(--border);
  overflow: hidden;
}

.bar-fill {
  width: 0;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
}

.bar-fill.running {
  animation: fill linear forwards;
}

.bar-fill.success {
  width: 100%;
  background: var(--success);
}

.bar-fill.failure {
  width: 100%;
  background: var(--error);
}

@keyframes fill {
  to {
    width: 95%;
  }
}
`

export const react19Template = {
  '/index.tsx': reactIndex,
  '/App.tsx': appTsx,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "react": "19.0.0",
        "react-dom": "19.0.0"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`
}

export const react18Template = {
  '/index.tsx': reactIndex,
  '/App.tsx': appTsx,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "react": "18.3.1",
        "react-dom": "18.3.1"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`
}

export const svelte5Template = {
  '/index.ts': indexSvelte,
  '/App.svelte': appSvelte,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "svelte": "^5.56.8"
    }
}`
}

export const vueTemplate = {
  '/index.ts': indexVue,
  '/App.vue': appVue,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "core-js": "3.26.1",
        "vue": "3.5.13"
    }
}`
}

export const FRAMEWORK_TEMPLATES = {
  react19: react19Template,
  react18: react18Template,
  svelte5: svelte5Template,
  vue: vueTemplate
}

export type FrameworkKey = keyof typeof FRAMEWORK_TEMPLATES

type StarterArg = { type: 'string' | 'number' }

function inlineRunnable(
  language: 'bun' | 'python3',
  content: string,
  args: Record<string, StarterArg>,
  fields: Record<string, any> = {}
) {
  return {
    fields,
    type: 'inline',
    inlineScript: {
      content,
      language,
      schema: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        properties: Object.fromEntries(
          Object.entries(args).map(([k, { type }]) => [
            k,
            { default: null, description: '', originalType: type, type }
          ])
        ),
        required: Object.keys(args),
        type: 'object'
      }
    }
  }
}

// The runnables every framework template calls, seeded by the apps_raw/edit page and
// the AI chat's app creation so the starter works on first render.
export const STARTER_RUNNABLES = {
  greet: {
    name: 'greet',
    ...inlineRunnable(
      'bun',
      `// import * as wmill from "windmill-client"

// \`username\` is a context field: the app fills it with the viewer's username.
// Direct API callers can pass any value, so don't use it for access control.
export async function main(name: string, username: string) {
  return {
    message: \`Hello \${name}! \${username ? \`You are signed in as \${username}.\` : 'You are not signed in.'}\`,
    ran_at: new Date().toISOString()
  }
}
`,
      { name: { type: 'string' }, username: { type: 'string' } },
      { username: { type: 'ctx', ctx: 'username' } }
    )
  },
  word_stats: {
    name: 'word_stats',
    ...inlineRunnable(
      'python3',
      `import re
from collections import Counter


def main(text: str):
    words = re.findall(r"[a-z']+", text.lower())
    top = Counter(w for w in words if len(w) > 4).most_common(5)
    return {
        "words": len(words),
        "characters": len(text),
        "sentences": len(re.findall(r"[.!?]+", text)),
        "top_words": [word for word, _ in top],
    }
`,
      { text: { type: 'string' } }
    )
  },
  slow_task: {
    name: 'slow_task',
    ...inlineRunnable(
      'bun',
      `export async function main(seconds: number) {
  await new Promise((resolve) => setTimeout(resolve, seconds * 1000))
  return \`Done after \${seconds}s\`
}
`,
      { seconds: { type: 'number' } }
    )
  },
  stream_text: {
    name: 'stream_text',
    ...inlineRunnable(
      'bun',
      `// Returning an async generator streams each yielded chunk to the caller.
export async function main() {
  const text =
    'Every word of this text is yielded by a Bun script as soon as it is ready, ' +
    'and the app renders it the moment it arrives. Use the same pattern to ' +
    'stream an LLM answer, a long report or live logs.'
  return (async function* () {
    for (const word of text.split(' ')) {
      await new Promise((resolve) => setTimeout(resolve, 60))
      yield word + ' '
    }
  })()
}
`,
      {}
    )
  }
}

export const appVueRouter = `
<template>
  <div class="container">
    <!-- Navigation tabs -->
    <nav class="tabs">
      <button 
        v-for="tab in tabs" 
        :key="tab.id"
        :class="{ active: currentTab === tab.id }"
        @click="changeTab(tab.id)"
      >
        {{ tab.name }}
      </button>
    </nav>

    <!-- Content sections -->
    <div class="content">
      <div v-if="currentTab === 'home'" class="tab-content">
        <h2>Home</h2>
        <p>Welcome to the home tab!</p>
        <!-- Nested navigation example -->
        <div class="sub-nav">
          <button 
            v-for="subItem in ['latest', 'popular']" 
            :key="subItem"
            :class="{ active: currentSort === subItem }"
            @click="changeSort(subItem)"
          >
            {{ subItem }}
          </button>
        </div>
      </div>

      <div v-else-if="currentTab === 'about'" class="tab-content">
        <h2>About</h2>
        <p>This is the about section</p>
      </div>

      <div v-else-if="currentTab === 'contact'" class="tab-content">
        <h2>Contact</h2>
        <p>Contact information here</p>
      </div>
    </div>

    <!-- Debug info -->
    <div class="debug">
      <p>Current Tab: {{ currentTab }}</p>
      <p>Current Sort: {{ currentSort }}</p>
      <p>Current Hash: {{ currentHash }}</p>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue';

// Available tabs
const tabs = [
  { id: 'home', name: 'Home' },
  { id: 'about', name: 'About' },
  { id: 'contact', name: 'Contact' }
];

const currentTab = ref('home');
const currentSort = ref('latest');
const currentHash = ref('');

// Navigation functions
function changeTab(tabId) {
  currentTab.value = tabId;
  updateHash();
}

function changeSort(sort) {
  currentSort.value = sort;
  updateHash();
}

function updateHash() {
  const params = new URLSearchParams();
  params.set('tab', currentTab.value);
  if (currentSort.value !== 'latest') {
    params.set('sort', currentSort.value);
  }
  window.location.hash = params.toString();
  currentHash.value = window.location.hash;
}

function parseHash() {
  const params = new URLSearchParams(window.location.hash.slice(1));
  currentTab.value = params.get('tab') || 'home';
  currentSort.value = params.get('sort') || 'latest';
  currentHash.value = window.location.hash;
}

// Hash change handler
function handleHashChange() {
  parseHash();
}

onMounted(() => {
  if (window.location.hash) {
    parseHash();
  } else {
    updateHash();
  }
  window.addEventListener('hashchange', handleHashChange);
});

onUnmounted(() => {
  window.removeEventListener('hashchange', handleHashChange);
});
</script>

<style scoped>
.container {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

.tabs {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

button {
  padding: 8px 16px;
  border: 1px solid #ddd;
  background: #fff;
  border-radius: 4px;
  cursor: pointer;
}

button.active {
  background: #4CAF50;
  color: white;
  border-color: #4CAF50;
}

.content {
  padding: 20px;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.tab-content {
  margin-bottom: 20px;
}

.sub-nav {
  margin-top: 10px;
}

.debug {
  margin-top: 20px;
  padding: 10px;
  background: #f5f5f5;
  border-radius: 4px;
  font-size: 0.9em;
  color: #666;
}
</style>`

export const appSvelteRouter = `
<script>
import { onMount, onDestroy } from 'svelte';

// Available tabs
const tabs = [
  { id: 'home', name: 'Home' },
  { id: 'about', name: 'About' },
  { id: 'contact', name: 'Contact' }
];

let currentTab = 'home';
let currentSort = 'latest';
let currentHash = '';

// Navigation functions
function changeTab(tabId) {
  currentTab = tabId;
  updateHash();
}

function changeSort(sort) {
  currentSort = sort;
  updateHash();
}

function updateHash() {
  const params = new URLSearchParams();
  params.set('tab', currentTab);
  if (currentSort !== 'latest') {
    params.set('sort', currentSort);
  }
  window.location.hash = params.toString();
  currentHash = window.location.hash;
}

function parseHash() {
  const params = new URLSearchParams(window.location.hash.slice(1));
  currentTab = params.get('tab') || 'home';
  currentSort = params.get('sort') || 'latest';
  currentHash = window.location.hash;
}

// Hash change handler
function handleHashChange() {
  parseHash();
}

onMount(() => {
  if (window.location.hash) {
    parseHash();
  } else {
    updateHash();
  }
  window.addEventListener('hashchange', handleHashChange);
});

onDestroy(() => {
  window.removeEventListener('hashchange', handleHashChange);
});
</script>

<div class="container">
  <!-- Navigation tabs -->
  <nav class="tabs">
    {#each tabs as tab}
      <button 
        class:active={currentTab === tab.id}
        on:click={() => changeTab(tab.id)}
      >
        {tab.name}
      </button>
    {/each}
  </nav>

  <!-- Content sections -->
  <div class="content">
    {#if currentTab === 'home'}
      <div class="tab-content">
        <h2>Home</h2>
        <p>Welcome to the home tab!</p>
        <!-- Nested navigation example -->
        <div class="sub-nav">
          {#each ['latest', 'popular'] as subItem}
            <button 
              class:active={currentSort === subItem}
              on:click={() => changeSort(subItem)}
            >
              {subItem}
            </button>
          {/each}
        </div>
      </div>
    {:else if currentTab === 'about'}
      <div class="tab-content">
        <h2>About</h2>
        <p>This is the about section</p>
      </div>
    {:else if currentTab === 'contact'}
      <div class="tab-content">
        <h2>Contact</h2>
        <p>Contact information here</p>
      </div>
    {/if}
  </div>

  <!-- Debug info -->
  <div class="debug">
    <p>Current Tab: {currentTab}</p>
    <p>Current Sort: {currentSort}</p>
    <p>Current Hash: {currentHash}</p>
  </div>
</div>

<style>
.container {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

.tabs {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

button {
  padding: 8px 16px;
  border: 1px solid #ddd;
  background: #fff;
  border-radius: 4px;
  cursor: pointer;
}

button.active {
  background: #4CAF50;
  color: white;
  border-color: #4CAF50;
}

.content {
  padding: 20px;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.tab-content {
  margin-bottom: 20px;
}

.sub-nav {
  margin-top: 10px;
}

.debug {
  margin-top: 20px;
  padding: 10px;
  background: #f5f5f5;
  border-radius: 4px;
  font-size: 0.9em;
  color: #666;
}
</style>`

export const appReactRouter = `
import React, { useState, useEffect } from 'react';

const tabs = [
  { id: 'home', name: 'Home' },
  { id: 'about', name: 'About' },
  { id: 'contact', name: 'Contact' }
];

const App = () => {
  const [currentTab, setCurrentTab] = useState('home');
  const [currentSort, setCurrentSort] = useState('latest');
  const [currentHash, setCurrentHash] = useState('');

  const updateHash = (newTab?: string, newSort?: string) => {
    const params = new URLSearchParams();
    params.set('tab', newTab ?? currentTab);
    if ((newSort ?? currentSort) !== 'latest') {
      params.set('sort', newSort ?? currentSort);
    }
    window.location.hash = params.toString();
    setCurrentHash(window.location.hash);
    if (newTab) setCurrentTab(newTab);
    if (newSort) setCurrentSort(newSort);
  };

  const parseHash = () => {
    const params = new URLSearchParams(window.location.hash.slice(1));
    setCurrentTab(params.get('tab') || 'home');
    setCurrentSort(params.get('sort') || 'latest');
    setCurrentHash(window.location.hash);
  };

  useEffect(() => {
    if (!window.location.hash) {
      updateHash(currentTab, currentSort);
    } else {
      parseHash();
    }

    const handleHashChange = () => parseHash();
    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  return (
    <div className="max-w-3xl mx-auto p-5">
      <nav className="flex gap-3 mb-5">
        {tabs.map(tab => (
          <button
            key={tab.id}
            onClick={() => updateHash(tab.id)}
            className={\`px-4 py-2 border rounded-md cursor-pointer
              $\{currentTab === tab.id 
                ? 'bg-green-500 text-white border-green-500' 
                : 'bg-white border-gray-300 hover:bg-gray-50'}\`}
          >
            {tab.name}
          </button>
        ))}
      </nav>

      <div className="p-5 border rounded-md">
        {currentTab === 'home' && (
          <div className="mb-5">
            <h2 className="text-xl font-bold">Home</h2>
            <p>Welcome to the home tab!</p>
            
            <div className="mt-3 flex gap-3">
              {['latest', 'popular'].map(sort => (
                <button
                  key={sort}
                  onClick={() => updateHash(currentTab, sort)}
                  className={\`px-4 py-2 border rounded-md cursor-pointer
                    $\{
											currentSort === sort
												? 'bg-green-500 text-white border-green-500'
												: 'bg-white border-gray-300 hover:bg-gray-50'
										}\`}
                >
                  {sort}
                </button>
              ))}
            </div>
          </div>
        )}

        {currentTab === 'about' && (
          <div>
            <h2 className="text-xl font-bold">About</h2>
            <p>This is the about section</p>
          </div>
        )}

        {currentTab === 'contact' && (
          <div>
            <h2 className="text-xl font-bold">Contact</h2>
            <p>Contact information here</p>
          </div>
        )}
      </div>

      <div className="mt-5 p-3 bg-gray-100 rounded-md text-sm text-gray-600">
        <p>Current Tab: {currentTab}</p>
        <p>Current Sort: {currentSort}</p>
        <p>Current Hash: {currentHash}</p>
      </div>
    </div>
  );
};

export default App
`
