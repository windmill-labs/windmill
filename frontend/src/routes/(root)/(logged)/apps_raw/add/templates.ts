const reactIndex = `
import { createRoot } from 'react-dom/client';
import './App'

const root = createRoot(document.getElementById('root')!);
root.render(<App />);
`

const appTsx = `import React from 'react';
import { runBg } from './wmill.ts'

const App = () => {
    return <div style={{ width: "100%" }}>
        <h1>Hello, Wooorldd!</h1>
        <div style={{ width: "100%", height: "100%", background: "red" }}>BAR</div>
    </div>;
};

async function m() {
    await runBg.a({ x: 42 })
}

m()
`

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

const app = mount(App, { target: document.getElementById("root") });

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

const indexCss = `body {
    background: blue;
}`

const policyJson = 'foo'

export const react19Template = {
	'/index.tsx': {
		code: reactIndex
	},
	'/App.tsx': {
		code: appTsx
	},
	'/index.css': {
		code: indexCss
	},
	'/package.json': {
		code: `{
    "dependencies": {
        "react": "19.0.0",
        "react-dom": "19.0.0"
    }
}`
	},
	'/policy.json': {
		code: policyJson
	}
}

export const react18Template = {
	'/index.tsx': {
		code: reactIndex
	},
	'/App.tsx': {
		code: appTsx
	},
	'/index.css': {
		code: indexCss
	},
	'/package.json': {
		code: `{
    "dependencies": {
        "react": "18.3.1",
        "react-dom": "18.3.1"
    }
}`
	},
	'/policy.json': {
		code: policyJson
	}
}

export const svelte5Template = {
	'/index.ts': {
		code: indexSvelte
	},
	'/App.svelte': {
		code: appSvelte
	},
	'/index.css': {
		code: indexCss
	},
	'/package.json': {
		code: `{
    "dependencies": {
        "svelte": "5.16.1"
    }
}`
	},
	'/policy.json': {
		code: policyJson
	}
}

export const vueTemplate = {
	'/index.ts': {
		code: indexVue
	},
	'/App.vue': {
		code: appVue
	},
	'/index.css': {
		code: indexCss
	},
	'/package.json': {
		code: `{
    "dependencies": {
        "core-js": "3.26.1",
        "vue": "3.2.45"
    }
}`
	},
	'/policy.json': {
		code: policyJson
	}
}
