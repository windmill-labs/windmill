// import "./style.css";
import { FlowBuilder } from "../lib/flow_builder";

import { workspaceStore, userStore } from "windmill-components/stores";

import React from "react";
import ReactDOM from "react-dom/client";
import { writable } from "svelte/store";
// To be able to test flows and run scripts, or get the app
// you will need to:
// 1. have a backend
// 2. for the token of the browser to contain a valid token wrt to this backend
// 3. That call to /api/* goes to the backend (in vite we use the proxy towards app.windmill.dev)

// Example on how to use the react SDK.

const workspace = "demo";
const username = "test";
const email = "test@test";

const flowStore = writable({
  summary: "",
  value: { modules: [] },
  path: "u/test/test",
  edited_at: "",
  edited_by: "",
  archived: false,
  extra_perms: {},
  schema: {},
});

userStore.set({
  username: username,
  email: email,
  is_admin: true,
  is_super_admin: true,
  operator: false,
  created_at: "",
  groups: [],
  pgroups: [],
  folders: [],
  folders_owners: [],
});

const flowStateStore = writable({});

workspaceStore.set(workspace);

ReactDOM.createRoot(document.getElementById("app") as HTMLElement).render(
  <React.StrictMode>
    <div className="p-4 border border-black embedded">
      <FlowBuilder flowStore={flowStore} flowStateStore={flowStateStore} />
    </div>
  </React.StrictMode>
);

// Example 2 on how to render an app

// import { AppService } from "windmill-client";
// import { AppPreview } from "../lib/app_viewer";

// const appPath = "f/examples/crm_gmail";
// const app = await AppService.getAppByPath({
//   workspace,
//   path: appPath,
// });

// ReactDOM.createRoot(document.getElementById("app") as HTMLElement).render(
//   <React.StrictMode>
//     <AppPreview
//       workspace={workspace}
//       app={app.value}
//       username={username}
//       email={email}
//       summary=""
//       appPath={appPath}
//     />
//   </React.StrictMode>
// );
