# Windmill Backend - Rust Best Practices

## Project Structure

Windmill uses a workspace-based architecture with multiple crates:

- **windmill-api**: API server functionality
- **windmill-worker**: Job execution
- **windmill-common**: Shared code used by all crates
- **windmill-queue**: Job & flow queuing
- **windmill-audit**: Audit logging
- Other specialized crates (git-sync, autoscaling, etc.)

## Adding New Code

### Module Organization

- Place new code in the appropriate crate based on functionality
- For API endpoints, create or modify files in `windmill-api/src/` organized by domain
- For shared functionality, use `windmill-common/src/`
- Use the `_ee.rs` suffix for enterprise-only modules
- Follow existing patterns for file structure and organization

### Error Handling

- Use the custom `Error` enum from `windmill-common::error`
- Return `Result<T, Error>` or `JsonResult<T>` for functions that can fail
- Use the `?` operator for error propagation
- Add location tracking to errors using `#[track_caller]`

### Database Operations

- Use `sqlx` for database operations with prepared statements
- Leverage existing database helper functions in `db.rs` modules
- Use transactions for multi-step operations
- Handle database errors properly

### API Endpoints

- Follow existing patterns in the `windmill-api` crate
- Use axum's routing system and extractors
- Group related routes together
- Use consistent response formats (JSON)
- Follow proper authentication and authorization patterns

## Performance Optimizations

When generating code, especially involving `serde`, `sqlx`, and `tokio`, prioritize performance by applying the following principles:

### Serde Optimizations (Serialization & Deserialization)

- **Specify Structure Explicitly:** When defining structs for Serde (`#[derive(Serialize, Deserialize)]`), use `#[serde(...` attributes extensively. This includes:
  - `#[serde(rename = "...")]` or `#[serde(alias = "...")]` to map external names precisely, avoiding dynamic lookups.
  - `#[serde(default)]` for optional fields with default values, reducing parsing complexity.
  - `#[serde(skip_serializing_if = "...")]` to avoid writing fields that meet a certain condition (e.g., `Option::is_none()`, `Vec::is_empty()`, or a custom function), reducing output size and serialization work.
  - `#[serde(skip_serializing)]` or `#[serde(skip_deserializing)]` for fields that should _not_ be included.
- **Prefer Borrowing:** Where possible and safe (data lifetime allows), use `Cow<'a, str>` or `&'a str` (with `#[serde(borrow)]`) instead of `String` for string fields during deserialization. This avoids allocating new strings, enabling zero-copy reading from the input buffer. Apply this principle to byte slices (`&'a [u8]` / `Cow<'a, [u8]>`) and potentially borrowed vectors as well.
- **Avoid Intermediate `Value`:** Unless the data structure is truly dynamic or unknown at compile time, deserialize directly into a well-defined struct or enum rather than into `serde_json::Value` (or equivalent for other formats). This avoids unnecessary heap allocations and type switching.

### SQLx Optimizations (Database Interaction)

- **Select Only Necessary Columns:** In `SELECT` queries, list specific column names rather than using `SELECT *`. This reduces data transferred from the database and the work needed for hydration/deserialization.
- **Batch Operations:** For multiple `INSERT`, `UPDATE`, or `DELETE` statements, prefer executing them in a single query if the database and driver support it efficiently (e.g., `INSERT INTO ... VALUES (...), (...), ...`). This minimizes round trips to the database.
- **Avoid N+1 Queries:** Do not loop through results of one query and execute a separate query for each item (e.g., fetching users, then querying for each user's profile in a loop). Instead, use JOINs or a single query with an `IN` clause to fetch related data efficiently.
- **Deserialize Directly:** Use `#[derive(FromRow)]` on structs and ensure the struct fields match the selected columns in the query. This allows SQLx to hydrate objects directly, avoiding intermediate data structures.
- **Parameterize Queries:** Always use SQLx's query methods (`.bind(...)`) to pass values as parameters rather than string formatting. This prevents SQL injection and allows the database to cache query plans, improving performance on repeated executions.

### Tokio Optimizations (Asynchronous Runtime)

- **Avoid Blocking Operations:** **Crucially**, never perform blocking operations (synchronous file I/O, `std::thread::sleep`, CPU-bound loops, `std::sync::Mutex::lock`, blocking network calls without `tokio::net`) directly within an `async fn` or a standard `tokio::spawn` task. Blocking pauses the entire worker thread, potentially starving other tasks. Use `tokio::task::spawn_blocking` for CPU-intensive work or blocking I/O.
- **Use Tokio's Async Primitives:** Prefer `tokio::sync` (channels, mutexes, semaphores), `tokio::io`, `tokio::net`, and `tokio::time` over their `std` counterparts in asynchronous contexts. These are designed to yield control back to the scheduler.
- **Manage Concurrency:** Be mindful of how many tasks are spawned. Creating a new task for every tiny piece of work can introduce overhead. Group related asynchronous operations where appropriate.
- **Handle Shared State Efficiently:** Use `Arc` for shared ownership in concurrent tasks. When shared state needs mutation, prefer `tokio::sync::Mutex` over `std::sync::Mutex` in `async` code. Consider `tokio::sync::RwLock` if reads significantly outnumber writes. Minimize the duration for which locks are held.
- **Understand `.await`:** Place `.await` strategically to allow the runtime to switch to other ready tasks. Ensure that `.await` points to genuinely asynchronous operations.
- **Backpressure:** If dealing with data streams or queues between tasks, implement backpressure mechanisms (e.g., bounded channels like `tokio::sync::mpsc::channel`) to prevent one component from overwhelming another or critical resources like the database.

## Enterprise Features

- Use feature flags for enterprise functionality
- Conditionally compile with `#[cfg(feature = "enterprise")]`
- Isolate enterprise code in separate modules

## Code Style

- Group imports by external and internal crates
- Place struct/enum definitions before implementations
- Group similar functionality together
- Use descriptive naming consistent with the codebase
- Follow existing patterns for async code using tokio

## Testing

- Write unit tests for core functionality
- Use the `#[cfg(test)]` module for test code
- For database tests, use the existing test utilities

## Common Crates Used

- **tokio**: For async runtime
- **axum**: For web server and routing
- **sqlx**: For database operations
- **serde**: For serialization/deserialization
- **tracing**: For logging and diagnostics
- **reqwest**: For HTTP client functionality

---

# Svelte 5 Best Practices

This guide outlines best practices for developing with Svelte 5, incorporating the new Runes API and other modern Svelte features. They should be applied on every new files created, but not on existing svelte 4 files unless specifically asked to.

## Reactivity with Runes

Svelte 5 introduces Runes for more explicit and flexible reactivity.

1.  **Embrace Runes for State Management**:

    - Use `$state` for reactive local component state.

      ```svelte
      <script>
        let count = $state(0);

        function increment() {
          count += 1;
        }
      </script>

      <button onclick={increment}>
        Clicked {count} {count === 1 ? 'time' : 'times'}
      </button>
      ```

    - Use `$derived` for computed values based on other reactive state.

      ```svelte
      <script>
        let count = $state(0);
        const doubled = $derived(count * 2);
      </script>

      <p>{count} * 2 = {doubled}</p>
      ```

    - Use `$effect` for side effects that need to run when reactive values change (e.g., logging, manual DOM manipulation, data fetching). Remember `$effect` does not run on the server.

      ```svelte
      <script>
        let count = $state(0);

        $effect(() => {
          console.log('The count is now', count);
          if (count > 5) {
            alert('Count is too high!');
          }
        });
      </script>
      ```

2.  **Props with `$props`**:

    - Declare component props using `$props()`. This offers better clarity and flexibility compared to `export let`.

      ```svelte
      <script>
        // ChildComponent.svelte
        let { name, age = $state(30) } = $props();
      </script>

      <p>Name: {name}</p>
      <p>Age: {age}</p>
      ```

    - For bindable props, use `$bindable`.

      ```svelte
      <script>
        // MyInput.svelte
        let { value = $bindable() } = $props();
      </script>

      <input bind:value />
      ```

## Event Handling

- **Use direct event attributes**: Svelte 5 moves away from `on:` directives for DOM events.
  - **Do**: `<button onclick={handleClick}>...</button>`
  - **Don't**: `<button on:click={handleClick}>...</button>`
- **For component events, prefer callback props**: Instead of `createEventDispatcher`, pass functions as props.

  ```svelte
  <!-- Parent.svelte -->
  <script>
    import Child from './Child.svelte';
    let message = $state('');
    function handleChildEvent(detail) {
      message = detail;
    }
  </script>
  <Child onCustomEvent={handleChildEvent} />
  <p>Message from child: {message}</p>

  <!-- Child.svelte -->
  <script>
    let { onCustomEvent } = $props();
    function emitEvent() {
      onCustomEvent('Hello from child!');
    }
  </script>
  <button onclick={emitEvent}>Send Event</button>
  ```

## Snippets for Content Projection

- **Use `{#snippet ...}` and `{@render ...}` instead of slots**: Snippets are more powerful and flexible.

  ```svelte
  <!-- Parent.svelte -->
  <script>
    import Card from './Card.svelte';
  </script>

  <Card>
    {#snippet title()}
      My Awesome Title
    {/snippet}
    {#snippet content()}
      <p>Some interesting content here.</p>
    {/snippet}
  </Card>

  <!-- Card.svelte -->
  <script>
    let { title, content } = $props();
  </script>

  <article>
    <header>{@render title()}</header>
    <div>{@render content()}</div>
  </article>
  ```

- Default content is passed via the `children` prop (which is a snippet).
  ```svelte
  <!-- Wrapper.svelte -->
  <script>
    let { children } = $props();
  </script>
  <div>
    {@render children?.()}
  </div>
  ```

## Component Design

1.  **Create Small, Reusable Components**: Break down complex UIs into smaller, focused components. Each component should have a single responsibility. This also aids performance by limiting the scope of reactivity updates.
2.  **Descriptive Naming**: Use clear and descriptive names for variables, functions, and components.
3.  **Minimize Logic in Components**: Move complex business logic to utility functions or services. Keep components focused on presentation and interaction.

## State Management (Stores)

1.  **Segment Stores**: Avoid a single global store. Create multiple stores, each responsible for a specific piece of global state (e.g., `userStore.js`, `themeStore.js`). This can help limit reactivity updates to only the parts of the UI that depend on specific state segments.
2.  **Use Custom Stores for Complex Logic**: For stores with related methods, create custom stores.

    ```javascript
    // counterStore.js
    import { writable } from "svelte/store";

    function createCounter() {
      const { subscribe, set, update } = writable(0);

      return {
        subscribe,
        increment: () => update((n) => n + 1),
        decrement: () => update((n) => n - 1),
        reset: () => set(0),
      };
    }
    export const counter = createCounter();
    ```

3.  **Use Context API for Localized State**: For state shared within a component subtree, consider Svelte's context API (`setContext`, `getContext`) instead of global stores when the state doesn't need to be truly global.

## Performance Optimizations (Svelte 5)

When generating Svelte 5 code, prioritize frontend performance by applying the following principles:

### General Svelte 5 Principles

- **Leverage the Compiler:** Trust Svelte's compiler to generate optimized JavaScript. Avoid manual DOM manipulation (`document.querySelector`, etc.) unless absolutely necessary for integrating third-party libraries that lack Svelte adapters.
- **Keep Components Small and Focused:** Reinforcing from Component Design, smaller components lead to less complex reactivity graphs and more targeted, efficient updates.

### Reactivity & State Management

- **Optimize Computations with `$derived`:** Always use `$derived` for computed values that depend on other state. This ensures the computation only runs when its specific dependencies change, avoiding unnecessary work compared to recomputing derived values in `$effect` or less efficient methods.
- **Minimize `$effect` Usage:** Use `$effect` sparingly and only for true side effects that interact with the outside world or non-Svelte state. Avoid putting complex logic or state updates _within_ an `$effect` unless those updates are explicitly intended as a reaction to external changes or non-Svelte state. Excessive or complex effects can impact rendering performance.
- **Structure State for Fine-Grained Updates:** Design your `$state` objects or variables such that updates affect only the necessary parts of the UI. Avoid putting too much unrelated state into a single large object that gets frequently updated, as this can potentially trigger broader updates than necessary. Consider normalizing complex, nested state.

### List Rendering (`{#each}`)

- **Mandate `key` Attribute:** Always use a `key` attribute (`{#each items as item (item.id)}`) that refers to a unique, stable identifier for each item in a list. This is critical for allowing Svelte to efficiently update, reorder, add, or remove list items without destroying and re-creating unnecessary DOM elements and component instances.

### Component Loading & Bundling

- **Implement Lazy Loading/Code Splitting:** For routes, components, or modules that are not immediately needed on page load, use dynamic imports (`import(...)`) to split the code bundle. SvelteKit handles this automatically for routes, but it can be applied manually to components using helper patterns if needed.
- **Be Mindful of Third-Party Libraries:** When incorporating external libraries, import only the necessary functions or components to minimize the final bundle size. Prefer libraries designed to be tree-shakeable.

### Rendering & DOM

- **Use CSS for Animations/Transitions:** Prefer CSS animations or transitions where possible for performance. Svelte's built-in `transition:` directive is also highly optimized and should be used for complex state-driven transitions, but simple cases can often use plain CSS.
- **Optimize Image Loading:** Implement best practices for images: use optimized formats (WebP, AVIF), lazy loading (`loading="lazy"`), and responsive images (`<picture>`, `srcset`) to avoid loading unnecessarily large images.

### Server-Side Rendering (SSR) & Hydration

- **Ensure SSR Compatibility:** Write components that can be rendered on the server for faster initial page loads. Avoid relying on browser-specific APIs (like `window` or `document`) in the main `<script>` context. If necessary, use `$effect` or check `if (browser)` inside effects to run browser-specific code only on the client.
- **Minimize Work During Hydration:** Structure components and data fetching such that minimal complex setup or computation is required when the client-side Svelte code takes over from the server-rendered HTML. Heavy synchronous work during hydration can block the main thread.

## General Clean Code Practices

1.  **Organized File Structure**: Group related files together. A common structure:
    ```
    /src
    |-- /routes      // Page components (if using a router like SvelteKit)
    |-- /lib         // Utility functions, services, constants (SvelteKit often uses this)
    |   |-- /stores
    |   |-- /utils
    |   |-- /services
    |   |-- /components  // Reusable UI components
    |-- App.svelte
    |-- main.js (or main.ts)
    ```
2.  **Scoped Styles**: Keep CSS scoped to components to avoid unintended side effects and improve maintainability. Avoid `:global` where possible.
3.  **Immutability**: With Svelte 5 and `$state`, direct assignments to properties of `$state` objects (`obj.prop = value;`) are generally fine as Svelte's reactivity system handles updates. However, for non-rune state or when interacting with other systems, understanding and sometimes preferring immutable updates (creating new objects/arrays) can still be relevant.
4.  **Use `class:` and `style:` directives**: For dynamic classes and styles, use Svelte's built-in directives for cleaner templates and potentially optimized updates.

    ```svelte
    <script>
      let isActive = $state(true);
      let color = $state('blue');
    </script>

    <div class:active={isActive} style:color={color}>
      Hello
    </div>
    ```

5.  **Stay Updated**: Keep Svelte and its related packages up to date to benefit from the latest features, performance improvements, and security fixes.
