# vindue.app website

Docusaurus site: landing page + docs, served at **https://vindue.app**.
Deployed by the `website` + `deploy` jobs in `.github/workflows/ci.yml` —
on pushes to `main` touching `website/**`, only after the `check` job
passes (docs never publish from a red commit).

```sh
npm install      # Node 22 (matches CI)
npm start        # local dev server with hot reload
npm run build    # static build into build/
```

Content lives in `docs/` (sidebar order via `_category_.json` +
`sidebar_position` front matter; the generated `http-api` category takes
its label/position from its `index.mdx`). `docs/reference/generated/` and
`docs/reference/http-api/` are **generated** from the Rust source — change
`api.rs`/`mcp.rs` and run `npm run docs:all` at the repo root; never
hand-edit them (CI drift gates fail). The root README points at this site
instead of duplicating it.
