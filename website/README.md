# vindue.app website

Docusaurus site: landing page + docs, deployed to GitHub Pages by
`.github/workflows/website.yml` on pushes to `main` touching `website/**`.

```sh
npm install
npm start        # local dev server with hot reload
npm run build    # static build into build/
```

Content lives in `docs/` (sidebar order via `_category_.json` +
`sidebar_position` front matter). Most docs are maintained alongside — and
kept consistent with — the root `README.md`.
