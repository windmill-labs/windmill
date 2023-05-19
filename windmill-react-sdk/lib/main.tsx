import FlowEditor from "windmill-components/components/FlowBuilder.svelte";
import React from "react";

import { reactify } from "svelte-preprocess-react";
import { writable } from "svelte/store";

const FlowViewerReact = reactify(FlowEditor);

export const flowStore = writable({
  summary: "",
  value: { modules: [] },
  path: "",
  edited_at: "",
  edited_by: "",
  archived: false,
  extra_perms: {},
  schema: {},
});
const flowStateStore = writable({});

export function MyComponent() {
  return (
    <FlowViewerReact
      flowStateStore={flowStateStore}
      flowStore={flowStore}
      selectedId={undefined}
    ></FlowViewerReact>
  );
}
