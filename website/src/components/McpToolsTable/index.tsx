import React from 'react';
import tools from '../../../docs/reference/generated/mcp-tools.json';

// The JSON is the committed output of `npm run docs:gen` (repo root) —
// exactly what the server's `tools/list` returns, dumped from the rmcp tool
// router (src-tauri/src/mcp.rs). A CI drift gate keeps it byte-identical to
// the code; this component just renders it, so the page can't go stale.

type ParamSchema = {type?: string; description?: string};

type ToolSchema = {
  type?: string;
  properties?: Record<string, ParamSchema>;
  required?: string[];
};

type Tool = {
  name: string;
  description?: string;
  inputSchema: ToolSchema;
};

function Params({schema}: {schema: ToolSchema}): React.JSX.Element {
  const props = schema.properties ?? {};
  const names = Object.keys(props);
  if (names.length === 0) {
    return <em>none</em>;
  }
  const required = new Set(schema.required ?? []);
  return (
    <ul>
      {names.map((n) => (
        <li key={n}>
          <code>{n}</code> <em>{props[n].type ?? 'any'}</em>,{' '}
          {required.has(n) ? 'required' : 'optional'}
          {props[n].description ? <> — {props[n].description}</> : null}
        </li>
      ))}
    </ul>
  );
}

export default function McpToolsTable(): React.JSX.Element {
  const sorted = [...(tools as unknown as Tool[])].sort((a, b) =>
    a.name.localeCompare(b.name),
  );
  return (
    <table>
      <thead>
        <tr>
          <th>Tool</th>
          <th>What it does</th>
          <th>Parameters</th>
        </tr>
      </thead>
      <tbody>
        {sorted.map((t) => (
          <tr key={t.name}>
            <td>
              <code>{t.name}</code>
            </td>
            <td>{t.description}</td>
            <td>
              <Params schema={t.inputSchema} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
