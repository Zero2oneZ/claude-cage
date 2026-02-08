use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Run a docker command and return stdout.
pub async fn docker(args: &[&str]) -> Result<String, String> {
    run("docker", args, None).await
}

/// List cage sessions via docker ps.
pub async fn list_sessions() -> Result<String, String> {
    docker(&[
        "ps", "-a",
        "--filter", "label=managed-by=claude-cage",
        "--format", "{{json .}}",
    ])
    .await
}

/// Get container inspect JSON.
pub async fn inspect_container(name: &str) -> Result<String, String> {
    docker(&["inspect", name]).await
}

/// Get container logs.
pub async fn container_logs(name: &str, tail: &str) -> Result<String, String> {
    docker(&["logs", "--tail", tail, name]).await
}

/// Stop a container.
pub async fn stop_container(name: &str) -> Result<String, String> {
    docker(&["stop", name]).await
}

/// Start a container.
pub async fn start_container(name: &str) -> Result<String, String> {
    docker(&["start", name]).await
}

/// Remove a container and its volumes.
pub async fn destroy_container(name: &str) -> Result<String, String> {
    docker(&["rm", "-f", "-v", name]).await
}

/// Create and start a new session via the cage CLI.
pub async fn create_session(
    cage_root: &Path,
    mode: &str,
    network: &str,
) -> Result<String, String> {
    let bin = cage_root.join("bin/claude-cage");
    run(
        bin.to_str().unwrap_or("claude-cage"),
        &["start", "--mode", mode, "--network", network],
        Some(cage_root),
    )
    .await
}

/// Query MongoDB via node store.js.
pub async fn mongo_get(
    store_js: &Path,
    collection: &str,
    query: &str,
    limit: u32,
) -> Result<String, String> {
    run(
        "node",
        &[
            store_js.to_str().unwrap_or("store.js"),
            "get",
            collection,
            query,
            &limit.to_string(),
        ],
        store_js.parent(),
    )
    .await
}

/// Put a document into MongoDB via node store.js.
pub async fn mongo_put(
    store_js: &Path,
    collection: &str,
    json: &str,
) -> Result<String, String> {
    run(
        "node",
        &[
            store_js.to_str().unwrap_or("store.js"),
            "put",
            collection,
            json,
        ],
        store_js.parent(),
    )
    .await
}

/// Log an event to MongoDB via node store.js.
pub async fn mongo_log(
    store_js: &Path,
    event_type: &str,
    key: &str,
    value: &str,
) -> Result<String, String> {
    run(
        "node",
        &[
            store_js.to_str().unwrap_or("store.js"),
            "log",
            event_type,
            key,
            value,
        ],
        store_js.parent(),
    )
    .await
}

/// Run PTC engine with an intent.
pub async fn ptc_run(
    cage_root: &Path,
    tree: &str,
    intent: &str,
) -> Result<String, String> {
    let tree_flag = format!("--tree={tree}");
    let intent_flag = format!("--intent={intent}");
    run(
        "python3",
        &["-m", "ptc.engine", &tree_flag, &intent_flag],
        Some(cage_root),
    )
    .await
}

/// Run PTC executor on a task JSON (passes input via stdin to avoid injection).
pub async fn ptc_execute(cage_root: &Path, task_json: &str) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;

    let mut child = Command::new("python3")
        .args([
            "-c",
            "import json,sys; sys.path.insert(0,'.'); from ptc.executor import execute; print(json.dumps(execute(json.loads(sys.stdin.read()))))",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(cage_root)
        .env("CAGE_ROOT", cage_root.to_str().unwrap_or("."))
        .spawn()
        .map_err(|e| format!("spawn python3: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(task_json.as_bytes())
            .await
            .map_err(|e| format!("write stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("wait python3: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("python3 failed ({}): {stderr}", output.status))
    }
}

/// Read the GentlyOS tree JSON from disk.
pub async fn read_tree(path: &Path) -> Result<serde_json::Value, String> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read tree: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse tree: {e}"))
}

/// Generic command runner.
async fn run(cmd: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut c = Command::new(cmd);
    c.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CAGE_ROOT", cwd.unwrap_or(Path::new(".")).to_str().unwrap_or("."));

    if let Some(dir) = cwd {
        c.current_dir(dir);
    }

    let output = c.output().await.map_err(|e| format!("spawn {cmd}: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("{cmd} failed ({}): {stderr} {stdout}", output.status))
    }
}
