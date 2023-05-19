import "./style.css";
import { FlowBuilder } from "../lib/flow_builder";
import { workspaceStore, userStore } from "windmill-components/stores";

import React from "react";
import ReactDOM from "react-dom/client";
import { writable } from "svelte/store";

export const flowStore = writable({
  summary: "",
  value: { modules: [] },
  path: "u/test/test",
  edited_at: "",
  edited_by: "",
  archived: false,
  extra_perms: {},
  schema: {},
});
const flowStateStore = writable({});
workspaceStore.set("test");
userStore.set({
  username: "test",
  email: "test@test",
  is_admin: true,
  is_super_admin: true,
  operator: false,
  created_at: "",
  groups: [],
  pgroups: [],
  folders: [],
  folders_owners: [],
});

ReactDOM.createRoot(document.getElementById("app") as HTMLElement).render(
  <React.StrictMode>
    <FlowBuilder flowStore={flowStore} flowStateStore={flowStateStore} />
  </React.StrictMode>
);
