import { useState, useRef, useEffect } from "react";

// Icons as SVG components (no external deps for reliability)
const Icon = ({ type, size = 18 }) => {
  const s = { width: size, height: size, fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" };
  const paths = {
    chat: <><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></>,
    globe: <><circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></>,
    folder: <><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></>,
    db: <><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></>,
    terminal: <><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></>,
    shield: <><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></>,
    send: <><line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/></>,
    plus: <><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></>,
    x: <><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></>,
    file: <><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></>,
    search: <><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></>,
    cpu: <><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></>,
    zap: <><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></>,
    eye: <><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></>,
    lock: <><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></>,
    save: <><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></>,
    play: <><polygon points="5 3 19 12 5 21 5 3"/></>,
    layers: <><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></>,
  };
  return <svg {...s} viewBox="0 0 24 24">{paths[type]}</svg>;
};

// Status indicator with shape (colorblind-safe)
const StatusDot = ({ status }) => {
  const colors = { active: "#3FB950", warning: "#D29922", error: "#F85149", idle: "#8B949E" };
  const shapes = { active: "●", warning: "▲", error: "◆", idle: "○" };
  return <span style={{ color: colors[status], fontSize: 10, marginRight: 4 }}>{shapes[status]}</span>;
};

export default function GentlyWorkstation() {
  // Layout state
  const [activeProject, setActiveProject] = useState(0);
  const [activeBrowserTab, setActiveBrowserTab] = useState(0);
  const [projectShelfExpanded, setProjectShelfExpanded] = useState(false);
  const [activeBottomPanel, setActiveBottomPanel] = useState("mongo");
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState([
    { role: "system", text: "GentlyOS Workstation v0.1 — Sovereignty Mode Active" },
    { role: "assistant", text: "3090 rig online. MongoDB logging active. What are we building?" },
  ]);
  const [browserUrl, setBrowserUrl] = useState("https://claude.ai");
  const [showGlobalSearch, setShowGlobalSearch] = useState(false);
  const chatEndRef = useRef(null);

  const projects = [
    { name: "GentlyOS", icon: "shield", status: "active", files: 147, branch: "main" },
    { name: "gently-goo", icon: "layers", status: "active", files: 23, branch: "dev" },
    { name: "Alexandria", icon: "search", status: "active", files: 56, branch: "main" },
    { name: "CODIE", icon: "zap", status: "warning", files: 12, branch: "feat/xor" },
    { name: "VirtOrg", icon: "cpu", status: "idle", files: 8, branch: "main" },
  ];

  const browserTabs = [
    { title: "Claude.ai", url: "https://claude.ai", favicon: "chat" },
    { title: "GitHub", url: "https://github.com/Zero2oneZ", favicon: "folder" },
    { title: "MongoDB Atlas", url: "https://cloud.mongodb.com", favicon: "db" },
    { title: "HuggingFace", url: "https://huggingface.co", favicon: "cpu" },
  ];

  const mongoLogs = [
    { ts: "02:14:33", type: "insert", col: "sessions", doc: "stage: 'build', artifacts: 3" },
    { ts: "02:14:31", type: "insert", col: "decisions", doc: "rated: 'mongodb-server', score: 7.5" },
    { ts: "02:14:28", type: "insert", col: "code", doc: "file: 'SKILL.md', lines: 380" },
    { ts: "02:13:55", type: "query", col: "sessions", doc: "filter: {stage: 'spark'}, found: 47" },
    { ts: "02:13:41", type: "insert", col: "ideas", doc: "name: 'mini-tom-lora', rating: 8.5" },
    { ts: "02:12:08", type: "insert", col: "sessions", doc: "stage: 'architecture', artifacts: 1" },
  ];

  const gpuStats = { gpu0: { temp: 62, util: 34, mem: "8.2/24GB", name: "RTX 3090 Ti #0" }, gpu1: { temp: 58, util: 12, mem: "3.1/24GB", name: "RTX 3090 Ti #1" } };

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatMessages]);

  const sendChat = () => {
    if (!chatInput.trim()) return;
    setChatMessages(prev => [...prev, { role: "user", text: chatInput }, { role: "assistant", text: "Processing... (Claude API + local model routing)" }]);
    setChatInput("");
  };

  const css = {
    root: { display: "flex", flexDirection: "column", height: "100vh", width: "100%", background: "#0D1117", color: "#E6EDF3", fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace", fontSize: 13, overflow: "hidden" },
    topNav: { display: "flex", alignItems: "center", height: 40, background: "#010409", borderBottom: "1px solid #21262D", padding: "0 12px", gap: 8, flexShrink: 0 },
    navBtn: { background: "none", border: "1px solid transparent", color: "#8B949E", padding: "4px 10px", borderRadius: 4, cursor: "pointer", fontSize: 11, display: "flex", alignItems: "center", gap: 4, transition: "all 150ms" },
    navBtnActive: { color: "#E6EDF3", borderColor: "#30363D", background: "#161B22" },
    main: { display: "flex", flex: 1, overflow: "hidden" },
    // Project shelf
    shelf: { width: projectShelfExpanded ? 220 : 48, background: "#010409", borderRight: "1px solid #21262D", display: "flex", flexDirection: "column", transition: "width 200ms cubic-bezier(0.4,0,0.2,1)", flexShrink: 0, overflow: "hidden" },
    shelfItem: (active) => ({ display: "flex", alignItems: "center", gap: 8, padding: projectShelfExpanded ? "8px 12px" : "8px 13px", cursor: "pointer", color: active ? "#58A6FF" : "#8B949E", background: active ? "#161B22" : "transparent", borderLeft: active ? "2px solid #58A6FF" : "2px solid transparent", transition: "all 150ms", whiteSpace: "nowrap", overflow: "hidden" }),
    // Chat panel
    chatPanel: { width: 320, display: "flex", flexDirection: "column", background: "#0D1117", borderRight: "1px solid #21262D", flexShrink: 0 },
    chatHeader: { padding: "8px 12px", borderBottom: "1px solid #21262D", display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: "#D2A8FF" },
    chatMessages: { flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 8 },
    chatBubble: (role) => ({ padding: "8px 12px", borderRadius: role === "user" ? "12px 12px 2px 12px" : "12px 12px 12px 2px", background: role === "user" ? "#1F3A5F" : role === "system" ? "#1A1A2E" : "#161B22", color: role === "system" ? "#D2A8FF" : "#E6EDF3", fontSize: 12, lineHeight: 1.5, maxWidth: "95%", alignSelf: role === "user" ? "flex-end" : "flex-start", border: role === "system" ? "1px solid #30363D" : "none" }),
    chatInputArea: { padding: 8, borderTop: "1px solid #21262D", display: "flex", gap: 6 },
    chatTextInput: { flex: 1, background: "#161B22", border: "1px solid #30363D", borderRadius: 6, padding: "6px 10px", color: "#E6EDF3", fontSize: 12, fontFamily: "inherit", outline: "none", resize: "none" },
    sendBtn: { background: "#58A6FF", border: "none", borderRadius: 6, padding: "6px 10px", color: "#0D1117", cursor: "pointer", display: "flex", alignItems: "center" },
    // Center area
    center: { flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" },
    // Browser
    browserBar: { display: "flex", alignItems: "center", background: "#010409", borderBottom: "1px solid #21262D", height: 36, flexShrink: 0 },
    browserTab: (active) => ({ display: "flex", alignItems: "center", gap: 4, padding: "0 12px", height: "100%", cursor: "pointer", color: active ? "#E6EDF3" : "#8B949E", background: active ? "#0D1117" : "transparent", borderBottom: active ? "2px solid #58A6FF" : "2px solid transparent", fontSize: 11, transition: "all 150ms", whiteSpace: "nowrap" }),
    browserAddTab: { padding: "0 8px", height: "100%", display: "flex", alignItems: "center", cursor: "pointer", color: "#8B949E" },
    urlBar: { display: "flex", alignItems: "center", padding: "4px 12px", background: "#0D1117", borderBottom: "1px solid #21262D", gap: 8 },
    urlInput: { flex: 1, background: "#161B22", border: "1px solid #30363D", borderRadius: 4, padding: "4px 8px", color: "#E6EDF3", fontSize: 11, fontFamily: "inherit", outline: "none" },
    browserContent: { flex: 1, background: "#161B22", display: "flex", alignItems: "center", justifyContent: "center", overflow: "auto" },
    // Bottom panel
    bottomBar: { display: "flex", background: "#010409", borderTop: "1px solid #21262D", borderBottom: "1px solid #21262D", height: 28, alignItems: "center", padding: "0 4px", flexShrink: 0 },
    bottomTab: (active) => ({ padding: "2px 10px", fontSize: 11, cursor: "pointer", color: active ? "#E6EDF3" : "#8B949E", background: active ? "#161B22" : "transparent", borderRadius: "4px 4px 0 0", display: "flex", alignItems: "center", gap: 4 }),
    bottomPanel: { height: 200, background: "#0D1117", borderTop: "1px solid #21262D", overflow: "auto", flexShrink: 0, fontSize: 11 },
    // Mongo table
    monoRow: { display: "flex", padding: "3px 12px", borderBottom: "1px solid #161B22", gap: 12 },
    monoCell: (w, color) => ({ width: w, color: color || "#8B949E", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }),
    // GPU panel
    gpuCard: { background: "#161B22", border: "1px solid #30363D", borderRadius: 6, padding: "8px 12px", flex: 1 },
    gpuBar: (pct) => ({ height: 4, borderRadius: 2, background: "#21262D", marginTop: 4, position: "relative", overflow: "hidden" }),
    gpuFill: (pct) => ({ position: "absolute", left: 0, top: 0, height: "100%", width: `${pct}%`, background: pct > 80 ? "#F85149" : pct > 50 ? "#D29922" : "#3FB950", borderRadius: 2, transition: "width 300ms" }),
    // Search overlay
    overlay: { position: "fixed", top: 0, left: 0, right: 0, bottom: 0, background: "rgba(1,4,9,0.8)", display: "flex", justifyContent: "center", paddingTop: 120, zIndex: 100 },
    searchBox: { width: 560, background: "#161B22", border: "1px solid #30363D", borderRadius: 10, padding: 4, height: "fit-content", boxShadow: "0 16px 48px rgba(0,0,0,0.4)" },
    searchInput: { width: "100%", background: "transparent", border: "none", padding: "12px 16px", color: "#E6EDF3", fontSize: 15, fontFamily: "inherit", outline: "none" },
  };

  return (
    <div style={css.root}>
      {/* ═══ TOP NAV BAR ═══ */}
      <div style={css.topNav}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginRight: 12 }}>
          <span style={{ color: "#D2A8FF", fontWeight: 700, fontSize: 13 }}>◆</span>
          <span style={{ color: "#E6EDF3", fontWeight: 600, fontSize: 12 }}>GentlyOS</span>
          <span style={{ color: "#3FB950", fontSize: 10, border: "1px solid #238636", padding: "1px 5px", borderRadius: 10 }}>● SOVEREIGN</span>
        </div>

        <button style={css.navBtn} onClick={() => setShowGlobalSearch(true)}>
          <Icon type="search" size={14} /> <span>Search</span>
          <span style={{ color: "#484F58", fontSize: 10, marginLeft: 4 }}>⌘K</span>
        </button>

        <div style={{ flex: 1 }} />

        {/* GPU status in nav */}
        <div style={{ display: "flex", gap: 12, alignItems: "center", fontSize: 10, color: "#8B949E" }}>
          <span><StatusDot status="active" />GPU0 {gpuStats.gpu0.temp}°C {gpuStats.gpu0.util}%</span>
          <span><StatusDot status="active" />GPU1 {gpuStats.gpu1.temp}°C {gpuStats.gpu1.util}%</span>
          <span style={{ color: "#3FB950" }}>▲ MongoDB</span>
          <span style={{ color: "#3FB950" }}>▲ Ollama</span>
        </div>

        <button style={css.navBtn}><Icon type="lock" size={14} /></button>
        <button style={css.navBtn}><Icon type="settings" size={14} /></button>
      </div>

      {/* ═══ MAIN AREA ═══ */}
      <div style={css.main}>
        {/* ─── PROJECT SHELF (left edge) ─── */}
        <div
          style={css.shelf}
          onMouseEnter={() => setProjectShelfExpanded(true)}
          onMouseLeave={() => setProjectShelfExpanded(false)}
        >
          <div style={{ padding: "10px 0 6px", borderBottom: "1px solid #21262D" }}>
            {projects.map((p, i) => (
              <div key={i} style={css.shelfItem(i === activeProject)} onClick={() => setActiveProject(i)}>
                <span style={{ flexShrink: 0 }}><Icon type={p.icon} size={16} /></span>
                {projectShelfExpanded && (
                  <div style={{ overflow: "hidden" }}>
                    <div style={{ fontSize: 12, fontWeight: i === activeProject ? 600 : 400 }}>
                      <StatusDot status={p.status} />{p.name}
                    </div>
                    <div style={{ fontSize: 10, color: "#484F58" }}>{p.branch} · {p.files} files</div>
                  </div>
                )}
              </div>
            ))}
          </div>
          <div style={{ marginTop: "auto", padding: 8, borderTop: "1px solid #21262D" }}>
            <div style={{ ...css.shelfItem(false), justifyContent: "center" }}>
              <Icon type="plus" size={14} />
              {projectShelfExpanded && <span style={{ fontSize: 11 }}>New Project</span>}
            </div>
          </div>
        </div>

        {/* ─── CLAUDE CHAT PANEL ─── */}
        <div style={css.chatPanel}>
          <div style={css.chatHeader}>
            <Icon type="chat" size={14} />
            <span style={{ fontWeight: 600 }}>Claude</span>
            <span style={{ fontSize: 10, color: "#8B949E", marginLeft: "auto" }}>Opus 4.5 · Max</span>
          </div>

          <div style={css.chatMessages}>
            {chatMessages.map((m, i) => (
              <div key={i} style={css.chatBubble(m.role)}>
                {m.role === "system" && <span style={{ fontSize: 10, display: "block", marginBottom: 2, color: "#D2A8FF" }}>◆ SYSTEM</span>}
                {m.text}
              </div>
            ))}
            <div ref={chatEndRef} />
          </div>

          <div style={css.chatInputArea}>
            <textarea
              style={css.chatTextInput}
              rows={2}
              value={chatInput}
              onChange={e => setChatInput(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendChat(); } }}
              placeholder="Message Claude..."
            />
            <button style={css.sendBtn} onClick={sendChat}>
              <Icon type="send" size={14} />
            </button>
          </div>

          {/* Chat context bar */}
          <div style={{ padding: "4px 8px", borderTop: "1px solid #21262D", display: "flex", gap: 4, flexWrap: "wrap" }}>
            <span style={{ fontSize: 10, color: "#484F58", padding: "2px 6px", background: "#161B22", borderRadius: 3, border: "1px solid #21262D" }}>
              📂 {projects[activeProject].name}
            </span>
            <span style={{ fontSize: 10, color: "#484F58", padding: "2px 6px", background: "#161B22", borderRadius: 3, border: "1px solid #21262D" }}>
              🔀 {projects[activeProject].branch}
            </span>
            <span style={{ fontSize: 10, color: "#3FB950", padding: "2px 6px", background: "#0D2818", borderRadius: 3, border: "1px solid #238636" }}>
              ● MongoDB logging
            </span>
          </div>
        </div>

        {/* ─── CENTER: BROWSER + BOTTOM PANEL ─── */}
        <div style={css.center}>
          {/* Browser tabs */}
          <div style={css.browserBar}>
            {browserTabs.map((t, i) => (
              <div key={i} style={css.browserTab(i === activeBrowserTab)} onClick={() => { setActiveBrowserTab(i); setBrowserUrl(t.url); }}>
                <Icon type={t.favicon} size={12} />
                <span>{t.title}</span>
                {i === activeBrowserTab && (
                  <span style={{ marginLeft: 4, color: "#484F58", cursor: "pointer" }} onClick={e => e.stopPropagation()}>
                    <Icon type="x" size={10} />
                  </span>
                )}
              </div>
            ))}
            <div style={css.browserAddTab}><Icon type="plus" size={12} /></div>
          </div>

          {/* URL bar */}
          <div style={css.urlBar}>
            <span style={{ color: "#3FB950" }}><Icon type="lock" size={12} /></span>
            <input
              style={css.urlInput}
              value={browserUrl}
              onChange={e => setBrowserUrl(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter") {} }}
            />
            <span style={{ color: "#8B949E", cursor: "pointer" }}><Icon type="eye" size={14} /></span>
          </div>

          {/* Browser content */}
          <div style={css.browserContent}>
            <div style={{ textAlign: "center", color: "#30363D" }}>
              <div style={{ fontSize: 48, marginBottom: 12 }}>◆</div>
              <div style={{ fontSize: 14, color: "#8B949E" }}>
                {browserTabs[activeBrowserTab]?.title || "New Tab"}
              </div>
              <div style={{ fontSize: 11, color: "#484F58", marginTop: 4 }}>
                {browserUrl}
              </div>
              <div style={{ marginTop: 24, display: "flex", gap: 8, justifyContent: "center", flexWrap: "wrap" }}>
                {["Claude.ai", "GitHub", "Docs", "HuggingFace", "MongoDB", "Ollama"].map((q, i) => (
                  <div key={i} style={{ padding: "6px 14px", background: "#161B22", border: "1px solid #30363D", borderRadius: 6, fontSize: 11, cursor: "pointer", color: "#8B949E" }}>
                    {q}
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* ─── BOTTOM PANEL BAR ─── */}
          <div style={css.bottomBar}>
            {[
              { id: "mongo", icon: "db", label: "MongoDB" },
              { id: "terminal", icon: "terminal", label: "Terminal" },
              { id: "gpu", icon: "cpu", label: "GPU Monitor" },
              { id: "files", icon: "file", label: "Files" },
              { id: "logs", icon: "eye", label: "Activity Log" },
            ].map(t => (
              <div key={t.id} style={css.bottomTab(activeBottomPanel === t.id)} onClick={() => setActiveBottomPanel(activeBottomPanel === t.id ? null : t.id)}>
                <Icon type={t.icon} size={12} />{t.label}
              </div>
            ))}
            <div style={{ flex: 1 }} />
            <span style={{ fontSize: 10, color: "#484F58", padding: "0 8px" }}>
              Session: 2h 14m · Interactions: 47 · Stage: BUILD
            </span>
          </div>

          {/* Bottom panel content */}
          {activeBottomPanel && (
            <div style={css.bottomPanel}>
              {activeBottomPanel === "mongo" && (
                <div>
                  <div style={{ ...css.monoRow, background: "#010409", position: "sticky", top: 0, fontWeight: 600, color: "#58A6FF" }}>
                    <span style={css.monoCell(70)}>TIME</span>
                    <span style={css.monoCell(60)}>OP</span>
                    <span style={css.monoCell(90)}>COLLECTION</span>
                    <span style={{ flex: 1, color: "#8B949E" }}>DOCUMENT</span>
                  </div>
                  {mongoLogs.map((log, i) => (
                    <div key={i} style={css.monoRow}>
                      <span style={css.monoCell(70, "#484F58")}>{log.ts}</span>
                      <span style={css.monoCell(60, log.type === "insert" ? "#3FB950" : "#D29922")}>{log.type}</span>
                      <span style={css.monoCell(90, "#D2A8FF")}>{log.col}</span>
                      <span style={{ flex: 1, color: "#E6EDF3" }}>{log.doc}</span>
                    </div>
                  ))}
                </div>
              )}

              {activeBottomPanel === "terminal" && (
                <div style={{ padding: 12, fontFamily: "inherit", whiteSpace: "pre" }}>
                  <div style={{ color: "#3FB950" }}>tom@gently-3090 <span style={{ color: "#58A6FF" }}>~/gentlyos</span> <span style={{ color: "#8B949E" }}>$</span></div>
                  <div style={{ color: "#E6EDF3", marginTop: 4 }}>Last login: Sat Feb 7 02:00:12 2026</div>
                  <div style={{ color: "#8B949E", marginTop: 4 }}>MongoDB: ● connected (atlas-cluster0)</div>
                  <div style={{ color: "#8B949E" }}>Ollama:   ● running (mistral:7b loaded)</div>
                  <div style={{ color: "#8B949E" }}>CUDA:     ● 12.4 (2x RTX 3090 Ti)</div>
                  <div style={{ marginTop: 8 }}>
                    <span style={{ color: "#3FB950" }}>tom@gently-3090</span>
                    <span style={{ color: "#58A6FF" }}> ~/gentlyos</span>
                    <span style={{ color: "#8B949E" }}> $ </span>
                    <span style={{ color: "#E6EDF3", borderRight: "2px solid #58A6FF", animation: "blink 1s step-end infinite", paddingRight: 2 }}>_</span>
                  </div>
                </div>
              )}

              {activeBottomPanel === "gpu" && (
                <div style={{ padding: 12, display: "flex", gap: 12 }}>
                  {Object.entries(gpuStats).map(([key, gpu]) => (
                    <div key={key} style={css.gpuCard}>
                      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                        <span style={{ fontWeight: 600, fontSize: 12 }}>{gpu.name}</span>
                        <span style={{ color: "#3FB950", fontSize: 11 }}>● Online</span>
                      </div>
                      <div style={{ display: "flex", gap: 16, fontSize: 11 }}>
                        <div style={{ flex: 1 }}>
                          <div style={{ color: "#8B949E", marginBottom: 2 }}>Utilization</div>
                          <div style={{ fontWeight: 600 }}>{gpu.util}%</div>
                          <div style={css.gpuBar(gpu.util)}><div style={css.gpuFill(gpu.util)} /></div>
                        </div>
                        <div style={{ flex: 1 }}>
                          <div style={{ color: "#8B949E", marginBottom: 2 }}>Temperature</div>
                          <div style={{ fontWeight: 600 }}>{gpu.temp}°C</div>
                          <div style={css.gpuBar(gpu.temp)}><div style={css.gpuFill(gpu.temp)} /></div>
                        </div>
                        <div style={{ flex: 1 }}>
                          <div style={{ color: "#8B949E", marginBottom: 2 }}>VRAM</div>
                          <div style={{ fontWeight: 600 }}>{gpu.mem}</div>
                          <div style={css.gpuBar(34)}><div style={css.gpuFill(34)} /></div>
                        </div>
                      </div>
                      <div style={{ marginTop: 8, display: "flex", gap: 6 }}>
                        <span style={{ fontSize: 10, padding: "2px 6px", background: "#0D2818", border: "1px solid #238636", borderRadius: 3, color: "#3FB950" }}>CUDA 12.4</span>
                        <span style={{ fontSize: 10, padding: "2px 6px", background: "#1A1A2E", border: "1px solid #30363D", borderRadius: 3, color: "#D2A8FF" }}>vLLM Ready</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {activeBottomPanel === "files" && (
                <div style={{ padding: "4px 0" }}>
                  {[
                    { name: "src/", type: "dir", mod: "2 min ago", size: "-" },
                    { name: "Cargo.toml", type: "file", mod: "1 hr ago", size: "2.4K" },
                    { name: "SKILL.md", type: "file", mod: "just now", size: "12K" },
                    { name: "idea-inventory.md", type: "file", mod: "just now", size: "8.6K" },
                    { name: ".claude/", type: "dir", mod: "5 min ago", size: "-" },
                    { name: "docker-compose.yml", type: "file", mod: "3 hrs ago", size: "1.1K" },
                  ].map((f, i) => (
                    <div key={i} style={{ ...css.monoRow, cursor: "pointer" }}>
                      <span style={css.monoCell(20, f.type === "dir" ? "#58A6FF" : "#8B949E")}>{f.type === "dir" ? "📁" : "📄"}</span>
                      <span style={{ flex: 1, color: f.type === "dir" ? "#58A6FF" : "#E6EDF3" }}>{f.name}</span>
                      <span style={css.monoCell(80, "#484F58")}>{f.mod}</span>
                      <span style={css.monoCell(50, "#484F58")}>{f.size}</span>
                    </div>
                  ))}
                </div>
              )}

              {activeBottomPanel === "logs" && (
                <div style={{ padding: 8, fontSize: 11, color: "#8B949E" }}>
                  <div><span style={{ color: "#3FB950" }}>[02:14:33]</span> <span style={{ color: "#D2A8FF" }}>MONGO</span> Inserted session document (stage: build)</div>
                  <div><span style={{ color: "#3FB950" }}>[02:14:31]</span> <span style={{ color: "#58A6FF" }}>CLAUDE</span> Rated idea: mongodb-server → 7.5/10</div>
                  <div><span style={{ color: "#3FB950" }}>[02:14:28]</span> <span style={{ color: "#D2A8FF" }}>MONGO</span> Inserted code artifact (SKILL.md, 380 lines)</div>
                  <div><span style={{ color: "#3FB950" }}>[02:13:55]</span> <span style={{ color: "#D29922" }}>QUERY</span> Searched sessions by stage:spark → 47 results</div>
                  <div><span style={{ color: "#3FB950" }}>[02:13:41]</span> <span style={{ color: "#58A6FF" }}>CLAUDE</span> New idea logged: mini-tom-lora (8.5/10)</div>
                  <div><span style={{ color: "#3FB950" }}>[02:12:08]</span> <span style={{ color: "#D2A8FF" }}>MONGO</span> Stage transition: architecture → build</div>
                  <div><span style={{ color: "#3FB950" }}>[02:10:22]</span> <span style={{ color: "#F85149" }}>GUARD</span> MVP flag: audio-diffusion marked as scope creep</div>
                  <div><span style={{ color: "#3FB950" }}>[02:08:15]</span> <span style={{ color: "#3FB950" }}>LORA</span> Weekly cook scheduled: Monday 02:00 UTC</div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* ═══ STATUS BAR ═══ */}
      <div style={{ display: "flex", alignItems: "center", height: 22, background: "#010409", borderTop: "1px solid #21262D", padding: "0 12px", fontSize: 10, color: "#484F58", gap: 16, flexShrink: 0 }}>
        <span style={{ color: "#D2A8FF" }}>◆ GentlyOS Workstation</span>
        <span><StatusDot status="active" />Sovereign Mode</span>
        <span>Project: {projects[activeProject].name}</span>
        <span>Branch: {projects[activeProject].branch}</span>
        <div style={{ flex: 1 }} />
        <span>MongoDB: 47 docs today</span>
        <span>LoRA: next cook Mon 02:00</span>
        <span>16hr limit: 13h 46m remaining</span>
      </div>

      {/* ═══ GLOBAL SEARCH OVERLAY ═══ */}
      {showGlobalSearch && (
        <div style={css.overlay} onClick={() => setShowGlobalSearch(false)}>
          <div style={css.searchBox} onClick={e => e.stopPropagation()}>
            <div style={{ display: "flex", alignItems: "center", padding: "0 12px" }}>
              <Icon type="search" size={16} />
              <input style={css.searchInput} autoFocus placeholder="Search everything — files, chats, MongoDB, ideas..." />
            </div>
            <div style={{ borderTop: "1px solid #21262D", padding: 8 }}>
              {["Recent: SKILL.md", "Recent: idea-inventory.md", "Idea: GOO (9.5/10)", "MongoDB: 47 sessions today", "Chat: mini-tom LoRA discussion"].map((item, i) => (
                <div key={i} style={{ padding: "6px 12px", borderRadius: 4, cursor: "pointer", fontSize: 12, color: "#8B949E" }}>
                  {item}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
