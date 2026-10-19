//! linear — Linear.app issue tools via GraphQL. Key from
//! LINEAR_API_KEY or ~/.gray/linear/config.json {"api_key","team"}.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

const NAME: &str = "linear";
fn api() -> String {
    std::env::var("LINEAR_API").unwrap_or_else(|_| "https://api.linear.app/graphql".into())
}

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Linear",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "linear_issues",
             "description": "List/search Linear issues. filter = substring match on title; state filters by state name; team limits to a team key. Returns 'KEY | state | title' lines.",
             "parameters": {"type": "object", "properties": {
                "filter": {"type": "string"}, "team": {"type": "string"},
                "state": {"type": "string"}, "limit": {"type": "integer"}}}},
            {"name": "linear_issue",
             "description": "Full detail on one issue by key (ABC-123) or id.",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}}, "required": ["key"]}},
            {"name": "linear_comment",
             "description": "Comment on an issue.",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}, "body": {"type": "string"}},
             "required": ["key","body"]}},
            {"name": "linear_create",
             "description": "Create an issue. team = team key (defaults to config team).",
             "parameters": {"type": "object", "properties": {
                "title": {"type": "string"}, "description": {"type": "string"},
                "team": {"type": "string"}, "priority": {"type": "integer"}},
             "required": ["title"]}},
        ],
        "commands": ["/linear"], "subcommands": ["linear"],
    })
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| std::path::PathBuf::from(h).join(".gray"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("linear").join("config.json")
}
fn read_config() -> Value {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}
fn api_key() -> Option<String> {
    std::env::var("LINEAR_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| read_config()["api_key"].as_str().map(str::to_string))
}

fn gql(query: &str, vars: Value) -> Result<Value, String> {
    let key = api_key().ok_or("no LINEAR_API_KEY — set env or run `linear setup --key …`")?;
    let resp = ureq::post(&api())
        .set("Authorization", &key)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send_json(json!({"query": query, "variables": vars}));
    match resp {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            if let Some(errs) = v["errors"].as_array() {
                return Err(errs
                    .iter()
                    .filter_map(|e| e["message"].as_str())
                    .collect::<Vec<_>>()
                    .join("; "));
            }
            Ok(v["data"].clone())
        }
        Err(ureq::Error::Status(c, r)) => Err(format!(
            "HTTP {c}: {}",
            r.into_string()
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect::<String>()
        )),
        Err(e) => Err(e.to_string()),
    }
}

fn fmt_issue(i: &Value) -> String {
    format!(
        "{} | {} | {}",
        i["identifier"].as_str().unwrap_or("?"),
        i["state"]["name"].as_str().unwrap_or("-"),
        i["title"].as_str().unwrap_or("")
    )
}

fn issues(a: &Value) -> Result<String, String> {
    let mut filter = json!({});
    if let Some(f) = a["filter"].as_str() {
        filter["title"] = json!({"containsIgnoreCase": f});
    }
    let team = a["team"]
        .as_str()
        .map(str::to_string)
        .or_else(|| read_config()["team"].as_str().map(str::to_string));
    if let Some(t) = team {
        filter["team"] = json!({"key": {"eq": t}});
    }
    if let Some(s) = a["state"].as_str() {
        filter["state"] = json!({"name": {"eqIgnoreCase": s}});
    }
    let limit = a["limit"].as_u64().unwrap_or(20);
    let q = r#"query($f: IssueFilter, $n: Int) {
        issues(filter: $f, first: $n, orderBy: updatedAt) {
            nodes { identifier title state { name } assignee { displayName } url }
        }
    }"#;
    let d = gql(q, json!({"f": filter, "n": limit}))?;
    let nodes = d["issues"]["nodes"].as_array().cloned().unwrap_or_default();
    if nodes.is_empty() {
        return Ok("no issues".into());
    }
    Ok(nodes
        .iter()
        .map(|i| {
            format!(
                "{}  {}",
                fmt_issue(i),
                i["assignee"]["displayName"]
                    .as_str()
                    .map(|a| format!(" @{a}"))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn issue(a: &Value) -> Result<String, String> {
    let key = a["key"].as_str().ok_or("key required")?;
    let q = r#"query($id: String!) {
        issue(id: $id) { identifier title description url
            state { name } assignee { displayName } team { key }
            comments { nodes { body user { displayName } createdAt } }
        }
    }"#;
    let d = gql(q, json!({"id": key}))?;
    let i = &d["issue"];
    if i.is_null() {
        return Err(format!("no issue {key}"));
    }
    let mut out = format!(
        "{} | {} | {}\n{}\n{}\n",
        i["identifier"].as_str().unwrap_or(""),
        i["state"]["name"].as_str().unwrap_or(""),
        i["title"].as_str().unwrap_or(""),
        i["url"].as_str().unwrap_or(""),
        i["description"].as_str().unwrap_or("")
    );
    for c in i["comments"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        out.push_str(&format!(
            "\n— {}: {}",
            c["user"]["displayName"].as_str().unwrap_or("?"),
            c["body"].as_str().unwrap_or("")
        ));
    }
    Ok(out)
}

fn comment(a: &Value) -> Result<String, String> {
    let key = a["key"].as_str().ok_or("key required")?;
    let body = a["body"].as_str().ok_or("body required")?;
    let d = gql(
        r#"query($id: String!) { issue(id: $id) { id } }"#,
        json!({"id": key}),
    )?;
    let iid = d["issue"]["id"].as_str().ok_or(format!("no issue {key}"))?;
    gql(
        r#"mutation($i: String!, $b: String!) {
        commentCreate(input: {issueId: $i, body: $b}) { success } }"#,
        json!({"i": iid, "b": body}),
    )?;
    Ok(format!("commented on {key}"))
}

fn create(a: &Value) -> Result<String, String> {
    let title = a["title"].as_str().ok_or("title required")?;
    let mut input = json!({"title": title});
    if let Some(d) = a["description"].as_str() {
        input["description"] = json!(d);
    }
    if let Some(p) = a["priority"].as_u64() {
        input["priority"] = json!(p);
    }
    // resolve team key → teamId
    let team = a["team"]
        .as_str()
        .map(str::to_string)
        .or_else(|| read_config()["team"].as_str().map(str::to_string));
    if let Some(t) = team {
        let d = gql(
            r#"query($k: String!) { teams(filter: {key: {eq: $k}}) { nodes { id } } }"#,
            json!({"k": t}),
        )?;
        let tid = d["teams"]["nodes"][0]["id"]
            .as_str()
            .ok_or(format!("no team {t}"))?;
        input["teamId"] = json!(tid);
    }
    let d = gql(
        r#"mutation($i: IssueCreateInput!) {
        issueCreate(input: $i) { success issue { identifier url } } }"#,
        json!({"i": input}),
    )?;
    let i = &d["issueCreate"]["issue"];
    Ok(format!(
        "{} {}",
        i["identifier"].as_str().unwrap_or("?"),
        i["url"].as_str().unwrap_or("")
    ))
}

fn tool(name: &str, args: &Value) -> Value {
    let r = match name {
        "linear_issues" => issues(args),
        "linear_issue" => issue(args),
        "linear_comment" => comment(args),
        "linear_create" => create(args),
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(c) => json!({"result": {"content": c}}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn cli(argv: &[String]) -> Result<String, String> {
    match argv.first().map(String::as_str) {
        Some("setup") => {
            let mut cfg = read_config();
            let mut it = argv[1..].iter();
            while let Some(f) = it.next() {
                match f.as_str() {
                    "--key" => cfg["api_key"] = json!(it.next().ok_or("--key needs value")?),
                    "--team" => cfg["team"] = json!(it.next().ok_or("--team needs value")?),
                    o => return Err(format!("unknown flag {o}")),
                }
            }
            std::fs::create_dir_all(config_path().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(config_path(), serde_json::to_string_pretty(&cfg).unwrap())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(config_path(), std::fs::Permissions::from_mode(0o600));
            }
            Ok("saved".into())
        }
        Some("status") | None => Ok(format!(
            "Linear\n  key: {}",
            if api_key().is_some() {
                "set"
            } else {
                "MISSING"
            }
        )),
        Some("-h" | "--help" | "help") => Ok("usage: linear setup | status".into()),
        Some(o) => Err(format!("unknown verb {o}")),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(
                        v["params"]["name"].as_str().unwrap_or(""),
                        &v["params"]["args"],
                    );
                    r["id"] = id;
                    r
                } else if method == "command/run" {
                    let argv: Vec<String> = v["params"]["argv"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();
                    let text = cli(&argv).unwrap_or_else(|e| e);
                    json!({"id": id, "result": {"text": text}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?;
                out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("linear") => args[1..].to_vec(),
        _ => args,
    };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => {
            let _ = sidecar();
        }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("linear — issue tools. `linear setup --key lin_api_… [--team ABC]`");
                    return;
                }
                let _ = sidecar();
                return;
            }
            match cli(&args) {
                Ok(t) => println!("{t}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
