import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

const sidebar: SidebarsConfig = {
  apisidebar: [
    {
      type: "doc",
      id: "reference/http-api/vindue-control-api",
    },
    {
      type: "category",
      label: "state",
      link: {
        type: "doc",
        id: "reference/http-api/state",
      },
      items: [
        {
          type: "doc",
          id: "reference/http-api/get-apps",
          label: "List running apps",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "reference/http-api/get-monitors",
          label: "List monitors",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "reference/http-api/get-state",
          label: "Live app state",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "reference/http-api/get-target",
          label: "Frontmost window",
          className: "api-method get",
        },
      ],
    },
    {
      type: "category",
      label: "tiling",
      link: {
        type: "doc",
        id: "reference/http-api/tiling",
      },
      items: [
        {
          type: "doc",
          id: "reference/http-api/tile",
          label: "Tile a window",
          className: "api-method post",
        },
      ],
    },
    {
      type: "category",
      label: "config",
      link: {
        type: "doc",
        id: "reference/http-api/config",
      },
      items: [
        {
          type: "doc",
          id: "reference/http-api/get-config",
          label: "Read full config",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "reference/http-api/put-api-config",
          label: "Update API settings",
          className: "api-method put",
        },
        {
          type: "doc",
          id: "reference/http-api/put-grid",
          label: "Update grid",
          className: "api-method put",
        },
        {
          type: "doc",
          id: "reference/http-api/put-keybindings",
          label: "Update keybindings",
          className: "api-method put",
        },
        {
          type: "doc",
          id: "reference/http-api/put-shortcut-assignment",
          label: "Update shortcut assignment",
          className: "api-method put",
        },
      ],
    },
    {
      type: "category",
      label: "shortcuts",
      link: {
        type: "doc",
        id: "reference/http-api/shortcuts",
      },
      items: [
        {
          type: "doc",
          id: "reference/http-api/get-shortcuts",
          label: "List shortcuts",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "reference/http-api/delete-shortcut",
          label: "Delete a shortcut",
          className: "api-method delete",
        },
        {
          type: "doc",
          id: "reference/http-api/put-shortcut",
          label: "Set a shortcut",
          className: "api-method put",
        },
      ],
    },
    {
      type: "category",
      label: "meta",
      link: {
        type: "doc",
        id: "reference/http-api/meta",
      },
      items: [
        {
          type: "doc",
          id: "reference/http-api/get-openapi",
          label: "This OpenAPI document",
          className: "api-method get",
        },
      ],
    },
  ],
};

export default sidebar.apisidebar;
