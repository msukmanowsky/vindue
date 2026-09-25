// Regenerates the committed doc artifacts from compiled code:
//
//   website/docs/reference/generated/openapi.json     — REST spec (utoipa,
//     annotated on the axum handlers in src/api.rs)
//   website/docs/reference/generated/mcp-tools.json   — MCP tool catalog
//     (exactly what `tools/list` serves clients, from the rmcp router in
//     src/mcp.rs)
//
// The CI drift gate re-runs this and fails on any `git diff` — the human docs
// cannot silently diverge from the code. From the repo root: `npm run
// docs:gen`, then `npm run docs:render` to re-render the website's endpoint
// pages from the spec.
use std::fs;
use std::path::PathBuf;

fn main() {
    let (openapi, tools) = vindue_lib::doc_artifacts();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../website/docs/reference/generated");
    fs::create_dir_all(&dir).expect("create generated/ dir");
    fs::write(dir.join("openapi.json"), openapi + "\n").expect("write openapi.json");
    fs::write(dir.join("mcp-tools.json"), tools + "\n").expect("write mcp-tools.json");
    eprintln!(
        "wrote {} and mcp-tools.json",
        dir.join("openapi.json").display()
    );
}
