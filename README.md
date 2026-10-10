<div align="center">
  <img alt="gray-linear" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-linear</h1>
  <p><strong>Linear issue tools for the agent — search, read, create, comment, and update issues.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-linear">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install linear
```

`gray-linear` exposes Linear's GraphQL API as agent tools. API key from
`LINEAR_API_KEY` or `~/.gray/linear/config.json` (`{"api_key","team"}`).

## Setup

```sh
linear setup --key lin_api_… [--team ENG]
linear status
```

## Tools

| Tool | What it does |
|---|---|
| `linear_issues` | List issues — `filter` (text), `team`, `state`, `limit`. |
| `linear_issue` | One issue by key (`ENG-123`). |
| `linear_create` | Create an issue — `title`, `description`, `team`, `priority`. Resolves the team by key first. |
| `linear_comment` | Comment on an issue — `key`, `body`. |

Missing credentials produce a clear `no LINEAR_API_KEY` error instead of
a network failure. `LINEAR_API` overrides the endpoint for testing or
proxies.

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/linear
```

## License

MIT
