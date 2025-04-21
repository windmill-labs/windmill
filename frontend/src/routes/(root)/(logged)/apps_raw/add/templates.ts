const reactIndex = `
import React from 'react'

import { createRoot } from 'react-dom/client'
import App from './App'

const root = createRoot(document.getElementById('root')!);
root.render(<App/>);
`
const appTsx = `import React, { useState } from 'react'
import { runBg } from './wmill'
import './index.css'

const App = () => {
    const [value, setValue] = useState(undefined as string | undefined)
    const [loading, setLoading] = useState(false)

    async function runA() {
        setLoading(true)
        try {
            setValue(await runBg.a({ x: 42 }))
        } catch (e) {
            console.error()
        }
        setLoading(false)
    }

    return <div style={{ width: "100%" }}>
        <h1>hello world</h1>

        <button style={{ marginTop: "2px" }} onClick={runA}>Run 'a'</button>

        <div style={{ marginTop: "20px", width: '250px' }} className='myclass'>
            {loading ? 'Loading ...' : value ?? 'Click button to see value here'}
        </div>
    </div>;
};

export default App;
`;

const appSvelte = `<style>
  h1 {
    font-size: 1.5rem;
  }
</style>

<main>
  <h1>Hello {name}</h1>
</main>

<script>
  let name = 'world';
</script>`

const indexSvelte = `
import { mount } from 'svelte';
import App from './App.svelte'
import './index.css'

const app = mount(App, { target: document.getElementById("root")! });

export default app;
`

const appVue = `<template>
  <h1>Hello {{ msg }}</h1>
</template>

<script setup>
import { ref } from 'vue';
const msg = ref('world');
</script>`

const indexVue = `import { createApp } from 'vue'
import App from './App.vue'
import "./index.css";

createApp(App).mount('#root')`

const indexCss = `.myclass {
    border: 1px solid gray;
    padding: 2px;
}`;

export const react19Template = {
  '/index.tsx': reactIndex,
  '/App.tsx': appTsx,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "react": "19.0.0",
        "react-dom": "19.0.0",
        "windmill-client": "^1"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`,
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
}`,
}

export const svelte5Template = {
  '/index.ts': indexSvelte,
  '/App.svelte': appSvelte,
  '/index.css': indexCss,
  '/package.json': `{
    "dependencies": {
        "svelte": "5.16.1",
        "windmill-client": "^1"
    }
}`,
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
}`,
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
