import AppPreview from "windmill-components/components/apps/editor/AppPreview.svelte";
// import "./app.css";

globalThis.render = (workspace) => {
  new AppPreview({
    target: document.getElementById("app"),
    props: {
      app: {
        grid: [
          {
            3: {
              fixed: true,
              x: 0,
              y: 0,
              fullHeight: false,
              w: 6,
              h: 2,
            },
            12: {
              fixed: true,
              x: 0,
              y: 0,
              fullHeight: false,
              w: 12,
              h: 2,
            },
            data: {
              type: "containercomponent",
              configuration: {},
              customCss: {
                container: {
                  class: "!p-0",
                  style: "",
                },
              },
              actions: [],
              numberOfSubgrids: 1,
              id: "topbar",
            },
            id: "topbar",
          },
          {
            3: {
              fixed: false,
              x: 0,
              y: 2,
              fullHeight: false,
              w: 3,
              h: 10,
            },
            12: {
              fixed: false,
              x: 0,
              y: 2,
              fullHeight: false,
              w: 6,
              h: 10,
            },
            data: {
              type: "aggridcomponent",
              configuration: {
                columnDefs: {
                  type: "static",
                  value: [
                    {
                      field: "id",
                      flex: 1,
                    },
                    {
                      field: "name",
                      editable: true,
                      flex: 1,
                    },
                    {
                      field: "age",
                      flex: 1,
                    },
                  ],
                },
                flex: {
                  type: "static",
                  value: true,
                },
                allEditable: {
                  type: "static",
                  value: false,
                },
                multipleSelectable: {
                  type: "static",
                  value: false,
                },
                rowMultiselectWithClick: {
                  type: "static",
                  value: true,
                },
                pagination: {
                  type: "static",
                  value: false,
                },
                selectFirstRowByDefault: {
                  type: "static",
                  value: true,
                },
                extraConfig: {
                  type: "static",
                  value: {},
                },
                compactness: {
                  type: "static",
                  value: "normal",
                },
                wrapActions: {
                  type: "static",
                  value: false,
                },
                footer: {
                  type: "static",
                  value: true,
                },
                customActionsHeader: {
                  type: "static",
                },
              },
              componentInput: {
                type: "static",
                fieldType: "array",
                subFieldType: "object",
                value: [
                  {
                    id: 1,
                    name: "A cell with a long name",
                    age: 42,
                  },
                  {
                    id: 2,
                    name: "A briefer cell",
                    age: 84,
                  },
                ],
              },
              customCss: {
                container: {
                  class: "",
                  style: "",
                },
              },
              actions: [],
              id: "a",
            },
            id: "a",
          },
        ],
        fullscreen: false,
        unusedInlineScripts: [],
        hiddenInlineScripts: [],
        theme: undefined,
      },
      author: "",
    },
  });
};

if (import.meta.env.DEV) {
  globalThis.render("foo");
}
