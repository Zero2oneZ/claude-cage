"""app.py — Flask web dashboard for claude-cage.

Serves the single-page dashboard and provides REST API endpoints
for container lifecycle, workspace management, and configuration.
"""

import json
import os
import subprocess
import sys
import threading

from flask import Flask, jsonify, request, render_template

# Add web/ to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import config_loader
import session_manager
import docker_manager
import workspace_manager

app = Flask(__name__)

CAGE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_cfg = None
_cfg_lock = threading.Lock()


def get_cfg():
    global _cfg
    with _cfg_lock:
        if _cfg is None:
            _cfg = config_loader.load_config(CAGE_ROOT)
        return _cfg


# ── Page routes ─────────────────────────────────────────────────


@app.route("/")
def index():
    return render_template("index.html")


# ── Health ──────────────────────────────────────────────────────


@app.route("/api/health")
def api_health():
    ok, msg = docker_manager.check()
    images = docker_manager.list_images()
    has_cli = any("cli" in i["tag"] for i in images)
    has_desktop = any("desktop" in i["tag"] for i in images)
    return jsonify({
        "docker": ok,
        "docker_message": msg,
        "images": {"cli": has_cli, "desktop": has_desktop},
        "api_key_set": bool(os.environ.get("ANTHROPIC_API_KEY")),
    })


# ── Sessions ────────────────────────────────────────────────────


@app.route("/api/sessions")
def api_list_sessions():
    cfg = get_cfg()
    sessions = session_manager.list_all(cfg)
    return jsonify(sessions)


@app.route("/api/sessions", methods=["POST"])
def api_create_session():
    cfg = get_cfg()
    data = request.get_json(force=True)

    mode = data.get("mode", "cli")
    name = data.get("name") or session_manager.generate_name()
    network = data.get("network", cfg.get("network", "filtered"))
    cpus = data.get("cpus", cfg.get("cpus", "2"))
    memory = data.get("memory", cfg.get("memory", "4g"))
    gpu = data.get("gpu", False)
    ephemeral = data.get("ephemeral", False)
    persist = data.get("persist", True)
    mounts = data.get("mounts", [])
    ports = data.get("ports", [])
    env_vars = data.get("env_vars", [])
    api_key = data.get("api_key") or os.environ.get("ANTHROPIC_API_KEY", "")

    if not api_key:
        return jsonify({"error": "ANTHROPIC_API_KEY is required"}), 400

    # Create session metadata
    session_manager.create(cfg, name, mode)

    # Touch workspaces
    for m in mounts:
        workspace_manager.touch_workspace(m)

    # Launch container
    ok, info = docker_manager.run_session(
        name=name, mode=mode, api_key=api_key, network=network,
        cpus=str(cpus), memory=str(memory), gpu=gpu,
        ephemeral=ephemeral, persist=persist, mounts=mounts,
        ports=ports, env_vars=env_vars, cage_root=CAGE_ROOT, cfg=cfg,
    )

    if ok:
        return jsonify(info), 201
    else:
        # Clean up metadata on failure
        session_manager.remove(cfg, name)
        return jsonify(info), 500


@app.route("/api/sessions/<name>")
def api_get_session(name):
    cfg = get_cfg()
    meta = session_manager.get_metadata(cfg, name)
    docker_info = docker_manager.inspect_session(name)
    if docker_info is None:
        return jsonify({"error": "Session not found"}), 404
    # Merge metadata with docker info
    result = {**meta, **docker_info}
    return jsonify(result)


@app.route("/api/sessions/<name>/stop", methods=["POST"])
def api_stop_session(name):
    cfg = get_cfg()
    ok, msg = docker_manager.stop_session(name)
    if ok:
        session_manager.set_status(cfg, name, "exited")
    return jsonify({"success": ok, "message": msg})


@app.route("/api/sessions/<name>/start", methods=["POST"])
def api_start_session(name):
    cfg = get_cfg()
    ok, msg = docker_manager.start_session(name)
    if ok:
        session_manager.set_status(cfg, name, "running")
    return jsonify({"success": ok, "message": msg})


@app.route("/api/sessions/<name>", methods=["DELETE"])
def api_destroy_session(name):
    cfg = get_cfg()
    force = request.args.get("force", "false") == "true"
    ok, msg = docker_manager.destroy_session(name, force=force)
    session_manager.remove(cfg, name)
    return jsonify({"success": ok, "message": msg})


@app.route("/api/sessions/<name>/logs")
def api_session_logs(name):
    tail = request.args.get("tail", 200, type=int)
    logs = docker_manager.get_logs(name, tail=tail)
    return jsonify({"logs": logs})


@app.route("/api/sessions/<name>/stats")
def api_session_stats(name):
    stats = docker_manager.get_stats(name)
    if stats is None:
        return jsonify({"error": "Not available"}), 404
    return jsonify(stats)


@app.route("/api/sessions/stop-all", methods=["POST"])
def api_stop_all():
    count = docker_manager.stop_all()
    return jsonify({"stopped": count})


# ── Workspaces ──────────────────────────────────────────────────


@app.route("/api/workspaces")
def api_list_workspaces():
    return jsonify(workspace_manager.list_workspaces())


@app.route("/api/workspaces", methods=["POST"])
def api_add_or_create_workspace():
    data = request.get_json(force=True)

    if "path" in data:
        ok, result = workspace_manager.add_workspace(data["path"])
    elif "name" in data:
        parent = data.get("parent")
        ok, result = workspace_manager.create_workspace(data["name"], parent)
    else:
        return jsonify({"error": "Provide 'path' or 'name'"}), 400

    if ok:
        return jsonify(result), 201
    else:
        return jsonify({"error": result}), 400


@app.route("/api/workspaces", methods=["DELETE"])
def api_remove_workspace():
    data = request.get_json(force=True)
    path = data.get("path")
    if not path:
        return jsonify({"error": "path required"}), 400
    ok, msg = workspace_manager.remove_workspace(path)
    return jsonify({"success": ok, "message": msg})


@app.route("/api/browse")
def api_browse_directory():
    path = request.args.get("path")
    entries = workspace_manager.browse_directory(path)
    parent = os.path.dirname(os.path.abspath(path)) if path else None
    return jsonify({"entries": entries, "current": path, "parent": parent})


# ── Build ───────────────────────────────────────────────────────


_build_status = {"running": False, "target": None, "message": ""}
_build_lock = threading.Lock()


@app.route("/api/build/<target>", methods=["POST"])
def api_build_image(target):
    cfg = get_cfg()

    with _build_lock:
        if _build_status["running"]:
            return jsonify({"error": "Build already in progress"}), 409
        _build_status["running"] = True
        _build_status["target"] = target
        _build_status["message"] = "Building..."

    def do_build():
        try:
            if target in ("cli", "all"):
                ok, msg = docker_manager.build_image("cli", CAGE_ROOT, cfg)
                if not ok:
                    with _build_lock:
                        _build_status["message"] = f"CLI build failed: {msg}"
                        _build_status["running"] = False
                    return
            if target in ("desktop", "all"):
                ok, msg = docker_manager.build_image("desktop", CAGE_ROOT, cfg)
                if not ok:
                    with _build_lock:
                        _build_status["message"] = f"Desktop build failed: {msg}"
                        _build_status["running"] = False
                    return
            with _build_lock:
                _build_status["message"] = "Build complete"
                _build_status["running"] = False
        except Exception as e:
            with _build_lock:
                _build_status["message"] = str(e)
                _build_status["running"] = False

    thread = threading.Thread(target=do_build, daemon=True)
    thread.start()
    return jsonify({"status": "started", "target": target})


@app.route("/api/build/status")
def api_build_status():
    with _build_lock:
        return jsonify(dict(_build_status))


@app.route("/api/images")
def api_list_images():
    return jsonify(docker_manager.list_images())


# ── Config ──────────────────────────────────────────────────────


@app.route("/api/config")
def api_get_config():
    cfg = get_cfg()
    return jsonify(config_loader.to_display(cfg))


# ── GentlyOS Tree ──────────────────────────────────────────────


@app.route("/api/gentlyos/tree")
def api_gentlyos_tree():
    tree_path = os.path.join(CAGE_ROOT, "gentlyos", "tree.json")
    if not os.path.exists(tree_path):
        return jsonify({"error": "tree.json not found"}), 404
    with open(tree_path) as f:
        return jsonify(json.load(f))


@app.route("/api/gentlyos/node/<node_id>")
def api_gentlyos_node(node_id):
    tree_path = os.path.join(CAGE_ROOT, "gentlyos", "tree.json")
    if not os.path.exists(tree_path):
        return jsonify({"error": "tree.json not found"}), 404
    with open(tree_path) as f:
        tree = json.load(f)
    for node in tree.get("nodes", []):
        if node["id"] == node_id:
            return jsonify(node)
    return jsonify({"error": f"Node {node_id} not found"}), 404


@app.route("/api/gentlyos/blast-radius")
def api_gentlyos_blast_radius():
    crates = request.args.get("crates", "").split(",")
    tree_path = os.path.join(CAGE_ROOT, "gentlyos", "tree.json")
    if not os.path.exists(tree_path):
        return jsonify({"error": "tree.json not found"}), 404
    with open(tree_path) as f:
        tree = json.load(f)
    nodes_by_id = {n["id"]: n for n in tree.get("nodes", [])}
    affected = set()
    for nid, node in nodes_by_id.items():
        owned = node.get("metadata", {}).get("crates_owned", [])
        if any(c in owned for c in crates) or "ALL" in owned:
            current = nid
            while current:
                affected.add(current)
                current = nodes_by_id.get(current, {}).get("parent")
    depts = [a for a in affected if a.startswith("dept:")]
    risk = min(10, len(depts) * 2 + 1)
    return jsonify({
        "affected_nodes": sorted(affected),
        "departments": sorted(depts),
        "risk_level": risk,
    })


# ── Blueprints (Architect Mode) ─────────────────────────────────


def _run_architect(args):
    """Run ptc.architect as subprocess, return parsed JSON or error."""
    cmd = [sys.executable, "-m", "ptc.architect"] + args
    env = {**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT}
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=30, env=env,
        )
        out = result.stdout.strip()
        if out:
            try:
                return json.loads(out), None
            except json.JSONDecodeError:
                return {"output": out}, None
        if result.returncode != 0:
            return None, result.stderr.strip() or "Command failed"
        return {"output": ""}, None
    except subprocess.TimeoutExpired:
        return None, "Timeout"
    except Exception as e:
        return None, str(e)


@app.route("/api/blueprints")
def api_list_blueprints():
    status_filter = request.args.get("status")
    args = ["list"]
    if status_filter:
        args += ["--status", status_filter]
    data, err = _run_architect(args)
    if err:
        return jsonify({"error": err}), 500
    return jsonify(data)


@app.route("/api/blueprints", methods=["POST"])
def api_create_blueprint():
    body = request.get_json(force=True)
    intent = body.get("intent", "")
    if not intent:
        return jsonify({"error": "intent required"}), 400
    data, err = _run_architect(["create", intent])
    if err:
        return jsonify({"error": err}), 500
    return jsonify(data), 201


@app.route("/api/blueprints/<bp_id>")
def api_get_blueprint(bp_id):
    data, err = _run_architect(["show", bp_id])
    if err:
        return jsonify({"error": err}), 404
    return jsonify(data)


@app.route("/api/blueprints/<bp_id>/build", methods=["POST"])
def api_build_blueprint(bp_id):
    data, err = _run_architect(["tasks", bp_id])
    if err:
        return jsonify({"error": err}), 500
    return jsonify(data)


@app.route("/api/blueprints/<bp_id>/verify", methods=["POST"])
def api_verify_blueprint(bp_id):
    data, err = _run_architect(["validate", bp_id])
    if err:
        return jsonify({"error": err}), 500
    return jsonify(data)


# ── Semantic Search ─────────────────────────────────────────────


@app.route("/api/search")
def api_search():
    query = request.args.get("q", "")
    if not query:
        return jsonify({"error": "q parameter required"}), 400
    cmd = [sys.executable, "-m", "ptc.embeddings", "search", query]
    env = {**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT}
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=30, env=env,
        )
        out = result.stdout.strip()
        if out:
            try:
                return jsonify(json.loads(out))
            except json.JSONDecodeError:
                return jsonify({"results": out})
        return jsonify({"results": []})
    except Exception as e:
        return jsonify({"error": str(e)}), 500


# ── IPFS Status ─────────────────────────────────────────────────


@app.route("/api/ipfs/status")
def api_ipfs_status():
    cmd = [sys.executable, "-m", "ptc.ipfs", "status"]
    env = {**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT}
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=10, env=env,
        )
        out = result.stdout.strip()
        if out:
            try:
                return jsonify(json.loads(out))
            except json.JSONDecodeError:
                return jsonify({"status": out})
        return jsonify({"status": "unknown"})
    except Exception as e:
        return jsonify({"error": str(e)}), 500


# ── Git Branches ────────────────────────────────────────────────


@app.route("/api/git/branches")
def api_git_branches():
    cmd = [sys.executable, "-m", "ptc.git_ops", "branches"]
    env = {**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT}
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=10, env=env,
            cwd=CAGE_ROOT,
        )
        out = result.stdout.strip()
        if out:
            try:
                return jsonify(json.loads(out))
            except json.JSONDecodeError:
                return jsonify({"branches": out.split("\n")})
        return jsonify({"branches": []})
    except Exception as e:
        return jsonify({"error": str(e)}), 500


# ── Main ────────────────────────────────────────────────────────


if __name__ == "__main__":
    port = int(os.environ.get("CAGE_WEB_PORT", 5000))
    print(f"claude-cage web dashboard: http://localhost:{port}")
    app.run(host="0.0.0.0", port=port, debug=False)
