import { sveltekit } from '@sveltejs/kit/vite';
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
    plugins: [sveltekit()],
    define: {
        __pkg__: version
    },
    ssr: {
        noExternal: ['dayjs']
    },
    optimizeDeps: {
        include: [
            'highlight.js',
            'highlight.js/lib/core',
        ]
    },
};

export default config;
