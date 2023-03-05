<script>
  import { findStore } from '../../store/controllers/storeApi';
  import { getNodeById } from '../../nodes/controllers/util';

  export let nodeId;
  export let canvasId;
  export let isEditing;

  let label;
  let width;
  let height;
  let customClass;
  let backgroundColor;

  const store = findStore(canvasId);

  const { nodesStore, nodeSelected } = store;
  let currentNode = $nodesStore[nodeId];

  const editNode = (e) => {
    e.preventDefault();
    if (label) currentNode.data.label = label;
    if (width) currentNode.width = width;
    if (height) currentNode.height = height;
    if (backgroundColor) currentNode.bgColor = backgroundColor;
    width = '';
    height = '';
    customClass = '';
    label = '';

    store.nodesStore.set($nodesStore);
  };
</script>

{#if isEditing}
  <div
    on:mouseover={(e) => ($nodeSelected = true)}
    on:focus
    on:wheel|preventDefault
    class="EditNode"
    style="left:{currentNode.positionX +
      currentNode.width}px; top:{currentNode.positionY}px"
  >
    <form on:submit={editNode}>
      <label for="label-input">Label</label>
      <input
        type="text"
        id="label-input-{nodeId}"
        placeholder={currentNode.data.label}
        bind:value={label}
      />

      <label for="bg-color-input">Background Color</label>
      <input
        type="color"
        id="bg-color-input-{nodeId}"
        class="bgci"
        bind:value={backgroundColor}
      />
      <input
        type="text"
        placeholder={currentNode.bgColor}
        bind:value={backgroundColor}
      />
    </form>
    <div class="btn-container">
      <button
        on:click={(e) => {
          const store = findStore(canvasId);
          const node = getNodeById(store, nodeId);
          node.delete();
          isEditing = false;
        }}>Delete Node</button
      >
      <button
        on:click={(e) => {
          editNode(e);
          isEditing = false;
        }}>Submit</button
      >
    </div>
  </div>
{/if}

<style>
  .EditNode {
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
    pointer-events: auto; /* this is needed for pointer events to work since we disable them in graphview */
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
