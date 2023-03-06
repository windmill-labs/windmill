<script lang="ts">
  import { findStore } from '../../store/controllers/storeApi';
  import { writable, derived, get, readable } from 'svelte/store';

  import { getEdgeById } from '../../edges/controllers/util';

  export let edgeId: string;
  export let canvasId: string;

  let label;
  let width;
  let height;
  let customClass;
  let backgroundColor;
  let type: 'straight' | 'smoothstep' | 'step' | 'bezier';

  const store = findStore(canvasId);
  const canvasWidth = get(store.widthStore);
  const canvasHeight = get(store.widthStore);
  const edge = getEdgeById(store, edgeId);
  const { edgesStore, nodeSelected } = store;

  function closeModal() {
    const store = findStore(canvasId);
    store.edgeEditModal.set(null);
  }
  function handleDelete() {
    const store = findStore(canvasId);
    const edge = getEdgeById(store, edgeId);
    edge.delete();
    closeModal();
  }
  function handleSubmit(e) {
    e.preventDefault();
    const store = findStore(canvasId);
    const edge = getEdgeById(store, edgeId);
    if (label) edge.label = label;
    if (backgroundColor) edge.edgeColor = backgroundColor;
    if (type) edge.type = type;
    // width = '';
    // height = '';
    // customClass = '';
    // label = '';
    store.edgesStore.set($edgesStore);
    closeModal();
  }
</script>

<div
  class="EditEdge"
  style="left:{canvasWidth / 3}px; top:{canvasHeight / 3}px"
  on:mouseover={(e) => ($nodeSelected = true)}
  on:focus
  on:wheel|preventDefault
>
  <form on:submit={handleSubmit}>
    <label for="label-input">Edge label</label>
    <input
      type="text"
      id="label-input-{edgeId}"
      placeholder={edge.label}
      bind:value={label}
    />

    <label for="bg-color-input">Edge color</label>
    <input
      type="color"
      id="bg-color-input-{edgeId}"
      class="bgci"
      bind:value={backgroundColor}
    />
    <input
      type="text"
      placeholder={edge.edgeColor}
      bind:value={backgroundColor}
    />

    <label for="type-input">Edge type</label>
    <input
      type="text"
      id="type-input-{edgeId}"
      placeholder={edge.type}
      bind:value={type}
    />
  </form>
  <div class="btn-container">
    <button on:click={handleSubmit}>Submit</button>
    <button on:click={handleDelete}>Delete</button>
    <button on:click={closeModal}>Cancel</button>
  </div>
</div>

<style>
  .EditEdge {
    position: absolute;
    padding: 0 1rem 1rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    border: 1px solid #333333;
    border-radius: 0.25rem;
    background-color: #ffffff;
    z-index: 10;
    user-select: text;
    box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.4);
    color: #333333;
  }

  label {
    font-size: 0.8rem;
    font-weight: bold;
    margin-bottom: 0.15rem;
  }

  .btn-container {
    display: flex;
    justify-content: space-between;
    margin-top: 0.5rem;
  }

  button {
    border-radius: 0.25rem;
    background-color: white;
    box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.4);
    margin: 0.2rem;
  }

  input {
    height: 1.6rem;
    border-color: #e45b56;
  }

  .bgci {
    height: 2rem;
    width: 5rem;
    padding: 0;
    border: none;
    background-color: none;
  }

  button:hover {
    cursor: pointer;
    background-color: #e45b56;
    color: white;
  }

  form {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
</style>
