# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend scripts and flows.

## Overview

Raw apps provide:
- Full control over the UI with your preferred framework
- Direct access to Windmill's API for running scripts and flows
- S3 integration for file handling
- Authentication through Windmill's user system

## File Structure

```
apps/
└── my_app/
    ├── index.html        # Entry point
    ├── app.js           # Application code
    ├── style.css        # Styles
    └── my_app.app.json  # Windmill metadata
```

## App Metadata

Create `{app_name}.app.json`:

```json
{
  "summary": "My Custom Dashboard",
  "policy": {
    "on_behalf_of_email": "",
    "execution_mode": "viewer"
  }
}
```

**Execution modes:**
- `viewer` - Run as the viewing user
- `publisher` - Run as the app publisher
- `anonymous` - Allow unauthenticated access

## Windmill Client SDK

Include the Windmill client in your app:

```html
<script src="https://app.windmill.dev/windmill_client.js"></script>
```

Or install via npm:
```bash
npm install windmill-client
```

## Running Scripts

```javascript
import { runScript } from 'windmill-client';

// Run a script and get the result
const result = await runScript({
  workspace: 'my_workspace',
  path: 'u/user/my_script',
  args: {
    param1: 'value1',
    param2: 42
  }
});

console.log(result);
```

## Running Flows

```javascript
import { runFlow } from 'windmill-client';

// Run a flow
const result = await runFlow({
  workspace: 'my_workspace',
  path: 'u/user/my_flow',
  args: {
    input_data: [1, 2, 3]
  }
});
```

## Getting Job Results

```javascript
import { getJobResult, waitForJobCompletion } from 'windmill-client';

// Start a job
const jobId = await runScriptAsync({
  workspace: 'my_workspace',
  path: 'u/user/long_running_script',
  args: {}
});

// Wait for completion
await waitForJobCompletion(jobId);

// Get the result
const result = await getJobResult(jobId);
```

## S3 File Operations

```javascript
import { loadS3File, writeS3File } from 'windmill-client';

// Download a file
const content = await loadS3File({
  s3: 'path/to/file.csv'
});

// Upload a file
const s3Object = await writeS3File(
  undefined,  // auto-generate path
  fileContent,
  's3_resource_path'
);
```

## Authentication Context

Access the current user information:

```javascript
import { getUser } from 'windmill-client';

const user = await getUser();
console.log(user.email, user.username);
```

## React Example

```jsx
import React, { useState, useEffect } from 'react';
import { runScript } from 'windmill-client';

function Dashboard() {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchData() {
      const result = await runScript({
        workspace: 'my_workspace',
        path: 'u/user/get_dashboard_data',
        args: {}
      });
      setData(result);
      setLoading(false);
    }
    fetchData();
  }, []);

  if (loading) return <div>Loading...</div>;

  return (
    <div>
      <h1>Dashboard</h1>
      <pre>{JSON.stringify(data, null, 2)}</pre>
    </div>
  );
}

export default Dashboard;
```

## Svelte Example

```svelte
<script>
  import { onMount } from 'svelte';
  import { runScript } from 'windmill-client';

  let data = null;
  let loading = true;

  onMount(async () => {
    data = await runScript({
      workspace: 'my_workspace',
      path: 'u/user/get_data',
      args: {}
    });
    loading = false;
  });
</script>

{#if loading}
  <p>Loading...</p>
{:else}
  <pre>{JSON.stringify(data, null, 2)}</pre>
{/if}
```

## Vue Example

```vue
<template>
  <div>
    <p v-if="loading">Loading...</p>
    <pre v-else>{{ JSON.stringify(data, null, 2) }}</pre>
  </div>
</template>

<script>
import { runScript } from 'windmill-client';

export default {
  data() {
    return {
      data: null,
      loading: true
    };
  },
  async mounted() {
    this.data = await runScript({
      workspace: 'my_workspace',
      path: 'u/user/get_data',
      args: {}
    });
    this.loading = false;
  }
};
</script>
```

## Error Handling

```javascript
try {
  const result = await runScript({
    workspace: 'my_workspace',
    path: 'u/user/my_script',
    args: {}
  });
} catch (error) {
  if (error.status === 401) {
    // Handle authentication error
  } else if (error.status === 404) {
    // Script not found
  } else {
    // Other error
    console.error(error.message);
  }
}
```

## Deployment

1. Bundle your app (Vite, Webpack, etc.)
2. Place built files in the app folder
3. Create the `.app.json` metadata file
4. Push with `wmill sync push`

## CLI Commands

```bash
# List apps
wmill app list

# Push app
wmill sync push

# Pull app
wmill sync pull
```
