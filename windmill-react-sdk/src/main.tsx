import "./style.css";
import { MyComponent } from "../lib/main";

import React from "react";
import ReactDOM from "react-dom/client";

ReactDOM.createRoot(document.getElementById("app") as HTMLElement).render(
  <React.StrictMode>
    <MyComponent />
  </React.StrictMode>
);
