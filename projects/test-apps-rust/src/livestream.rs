//! Model Livestreaming Service — profiles, live streams, chat, tips, scheduling.
//! Pure Rust, server-rendered HTML, in-memory state, zero JS frameworks.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct Model {
    id: u32,
    username: String,
    display_name: String,
    bio: String,
    avatar: String,
    category: String,
    followers: u32,
    is_live: bool,
    stream_title: String,
    viewers: u32,
    total_streams: u32,
    total_hours: f32,
    tags: Vec<String>,
    tier: String,
}

#[derive(Clone)]
struct ChatMessage {
    username: String,
    message: String,
    timestamp: String,
    is_tip: bool,
    tip_amount: Option<f64>,
}

#[derive(Clone)]
struct ScheduledStream {
    model_id: u32,
    title: String,
    scheduled_time: String,
    duration_hours: f32,
    category: String,
}

struct AppState {
    models: RwLock<Vec<Model>>,
    chats: RwLock<HashMap<u32, Vec<ChatMessage>>>,
    schedule: RwLock<Vec<ScheduledStream>>,
    total_tips: RwLock<f64>,
}

fn seed_models() -> Vec<Model> {
    vec![
        Model { id: 1, username: "synthwave_stella".into(), display_name: "Stella Neon".into(), bio: "Synthwave DJ & visual artist. Live sets every Fri/Sat. Retro-futurism meets modern bass.".into(), avatar: "🎧".into(), category: "Music".into(), followers: 45200, is_live: true, stream_title: "Friday Night Synthwave Session".into(), viewers: 1342, total_streams: 312, total_hours: 1248.5, tags: vec!["synthwave".into(), "dj".into(), "electronic".into()], tier: "Diamond".into() },
        Model { id: 2, username: "chef_marco".into(), display_name: "Chef Marco".into(), bio: "Michelin-trained chef. Italian & fusion cuisine. Cooking streams daily at noon.".into(), avatar: "👨‍🍳".into(), category: "Cooking".into(), followers: 28900, is_live: true, stream_title: "Making Fresh Pasta from Scratch".into(), viewers: 856, total_streams: 198, total_hours: 594.0, tags: vec!["cooking".into(), "italian".into(), "pasta".into()], tier: "Gold".into() },
        Model { id: 3, username: "code_ninja_dev".into(), display_name: "Alex DevOps".into(), bio: "Full-stack developer. Building production systems live. Rust, Go, TypeScript.".into(), avatar: "💻".into(), category: "Tech".into(), followers: 67800, is_live: true, stream_title: "Building a Distributed Cache in Rust".into(), viewers: 2105, total_streams: 445, total_hours: 1780.0, tags: vec!["rust".into(), "coding".into(), "systems".into()], tier: "Diamond".into() },
        Model { id: 4, username: "yoga_with_maya".into(), display_name: "Maya Flow".into(), bio: "Certified yoga instructor. Morning flows, meditation, breathwork. Mind-body connection.".into(), avatar: "🧘".into(), category: "Fitness".into(), followers: 34100, is_live: false, stream_title: "".into(), viewers: 0, total_streams: 267, total_hours: 534.0, tags: vec!["yoga".into(), "meditation".into(), "wellness".into()], tier: "Gold".into() },
        Model { id: 5, username: "pixel_artisan".into(), display_name: "Pixel Pete".into(), bio: "Retro pixel art & game design. Creating worlds one pixel at a time. Commissions open.".into(), avatar: "🎨".into(), category: "Art".into(), followers: 19500, is_live: true, stream_title: "Designing a Boss Sprite — 64x64 Challenge".into(), viewers: 673, total_streams: 156, total_hours: 624.0, tags: vec!["pixelart".into(), "gamedev".into(), "art".into()], tier: "Silver".into() },
        Model { id: 6, username: "astro_sarah".into(), display_name: "Sarah Stars".into(), bio: "Astrophysicist & telescope operator. Night sky tours, planet hunting, space science.".into(), avatar: "🔭".into(), category: "Science".into(), followers: 52300, is_live: false, stream_title: "".into(), viewers: 0, total_streams: 89, total_hours: 267.0, tags: vec!["astronomy".into(), "science".into(), "space".into()], tier: "Gold".into() },
        Model { id: 7, username: "beat_machine".into(), display_name: "BeatMachine".into(), bio: "Hip-hop producer. Making beats live, sample chopping, mixing & mastering tutorials.".into(), avatar: "🎹".into(), category: "Music".into(), followers: 41200, is_live: true, stream_title: "Lo-Fi Beat Making Marathon".into(), viewers: 987, total_streams: 278, total_hours: 834.0, tags: vec!["hiphop".into(), "beats".into(), "producer".into()], tier: "Gold".into() },
        Model { id: 8, username: "speed_runner_x".into(), display_name: "SpeedX".into(), bio: "Speedrunner & competitive gamer. World record holder in 3 categories. GDQ veteran.".into(), avatar: "🏃".into(), category: "Gaming".into(), followers: 89400, is_live: false, stream_title: "".into(), viewers: 0, total_streams: 512, total_hours: 2048.0, tags: vec!["speedrun".into(), "gaming".into(), "competitive".into()], tier: "Diamond".into() },
        Model { id: 9, username: "plant_parent".into(), display_name: "Ivy Green".into(), bio: "Urban gardener & plant care specialist. 200+ plants in my apartment. Propagation streams.".into(), avatar: "🌿".into(), category: "Lifestyle".into(), followers: 15800, is_live: true, stream_title: "Repotting Sunday — Monstera Edition".into(), viewers: 412, total_streams: 134, total_hours: 268.0, tags: vec!["plants".into(), "gardening".into(), "lifestyle".into()], tier: "Silver".into() },
        Model { id: 10, username: "math_wizard".into(), display_name: "Dr. Numbers".into(), bio: "PhD mathematician. Problem solving, proofs, competitive math coaching. Making math fun.".into(), avatar: "🧮".into(), category: "Education".into(), followers: 23700, is_live: false, stream_title: "".into(), viewers: 0, total_streams: 201, total_hours: 603.0, tags: vec!["math".into(), "education".into(), "tutoring".into()], tier: "Silver".into() },
        Model { id: 11, username: "forge_master".into(), display_name: "Iron Mike".into(), bio: "Blacksmith & metalworker. Forging knives, tools, and art. Real fire, real steel.".into(), avatar: "⚒️".into(), category: "Crafts".into(), followers: 37600, is_live: true, stream_title: "Forging a Damascus Steel Chef Knife".into(), viewers: 1567, total_streams: 167, total_hours: 668.0, tags: vec!["blacksmith".into(), "forging".into(), "crafts".into()], tier: "Gold".into() },
        Model { id: 12, username: "drone_pilot_z".into(), display_name: "SkyView".into(), bio: "FPV drone pilot & cinematographer. Cinematic flights, racing, freestyle. FAA Part 107.".into(), avatar: "🚁".into(), category: "Tech".into(), followers: 29100, is_live: false, stream_title: "".into(), viewers: 0, total_streams: 98, total_hours: 196.0, tags: vec!["drone".into(), "fpv".into(), "cinematography".into()], tier: "Silver".into() },
    ]
}

fn seed_schedule() -> Vec<ScheduledStream> {
    vec![
        ScheduledStream { model_id: 4, title: "Morning Vinyasa Flow".into(), scheduled_time: "2026-02-09T07:00:00Z".into(), duration_hours: 1.0, category: "Fitness".into() },
        ScheduledStream { model_id: 6, title: "Jupiter Opposition Watch Party".into(), scheduled_time: "2026-02-09T21:00:00Z".into(), duration_hours: 3.0, category: "Science".into() },
        ScheduledStream { model_id: 8, title: "SM64 16-Star Speedrun Attempts".into(), scheduled_time: "2026-02-09T14:00:00Z".into(), duration_hours: 6.0, category: "Gaming".into() },
        ScheduledStream { model_id: 10, title: "IMO Problem Set Walkthrough".into(), scheduled_time: "2026-02-10T18:00:00Z".into(), duration_hours: 2.0, category: "Education".into() },
        ScheduledStream { model_id: 12, title: "Sunset Beach FPV Session".into(), scheduled_time: "2026-02-10T16:30:00Z".into(), duration_hours: 1.5, category: "Tech".into() },
    ]
}

const CSS: &str = r#"
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d1117;color:#c9d1d9;font-family:'Segoe UI',system-ui,sans-serif}
a{color:#58a6ff;text-decoration:none}a:hover{text-decoration:underline}
.nav{background:#161b22;border-bottom:1px solid #30363d;padding:0.75rem 2rem;display:flex;align-items:center;gap:2rem}
.nav h1{font-size:1.2rem;color:#f0f6fc}.nav a{color:#8b949e}.nav a:hover{color:#f0f6fc}
.container{max-width:1200px;margin:0 auto;padding:1.5rem}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:1.25rem}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;overflow:hidden;transition:border-color .2s}
.card:hover{border-color:#58a6ff}
.stream-preview{background:linear-gradient(135deg,#1a1a2e 0%,#16213e 100%);padding:2.5rem;text-align:center;position:relative}
.stream-preview .avatar{font-size:4rem}
.live-badge{position:absolute;top:12px;left:12px;background:#da3633;color:#fff;padding:3px 10px;border-radius:4px;font-size:0.75rem;font-weight:700;animation:pulse 2s infinite}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:0.7}}
.viewer-count{position:absolute;top:12px;right:12px;background:rgba(0,0,0,0.7);color:#c9d1d9;padding:3px 10px;border-radius:4px;font-size:0.75rem}
.card-body{padding:1rem}
.card-body h3{color:#f0f6fc;font-size:1rem;margin-bottom:0.25rem}
.card-body .username{color:#8b949e;font-size:0.85rem}
.card-body .stream-title{color:#c9d1d9;font-size:0.9rem;margin:0.5rem 0}
.card-body .meta{color:#6e7681;font-size:0.8rem;display:flex;justify-content:space-between;margin-top:0.5rem}
.tags{display:flex;gap:0.4rem;flex-wrap:wrap;margin-top:0.5rem}
.tag{background:#21262d;color:#8b949e;padding:2px 8px;border-radius:4px;font-size:0.75rem}
.tier-diamond{color:#b388ff}.tier-gold{color:#ffd700}.tier-silver{color:#aaa}
.btn{background:#238636;color:#fff;border:none;padding:0.5rem 1rem;border-radius:6px;cursor:pointer;font-size:0.9rem}
.btn:hover{background:#2ea043}.btn-tip{background:#8b5cf6}.btn-tip:hover{background:#7c3aed}
.btn-outline{background:transparent;border:1px solid #30363d;color:#c9d1d9}.btn-outline:hover{border-color:#58a6ff}
input,select,textarea{background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:6px;padding:0.5rem;font-size:0.9rem;width:100%}
.hero{background:linear-gradient(135deg,#1a1a2e 0%,#0d1117 100%);border:1px solid #30363d;border-radius:12px;padding:2.5rem;margin-bottom:2rem}
.stats-row{display:grid;grid-template-columns:repeat(4,1fr);gap:1rem;margin-bottom:2rem}
.stat-card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:1.25rem;text-align:center}
.stat-card .number{font-size:1.8rem;font-weight:700;color:#f0f6fc}.stat-card .label{color:#8b949e;font-size:0.85rem}
.chat-box{background:#0d1117;border:1px solid #30363d;border-radius:8px;max-height:400px;overflow-y:auto;padding:1rem}
.chat-msg{padding:0.5rem 0;border-bottom:1px solid #21262d}
.chat-msg .name{color:#58a6ff;font-weight:600;font-size:0.85rem}
.chat-msg .text{color:#c9d1d9;font-size:0.9rem}
.chat-msg.tip{background:rgba(139,92,246,0.1);padding:0.5rem;border-radius:6px;margin:0.25rem 0}
.chat-msg.tip .tip-amount{color:#8b5cf6;font-weight:700}
.schedule-item{display:flex;align-items:center;gap:1rem;padding:0.75rem 0;border-bottom:1px solid #21262d}
.schedule-time{color:#3fb950;font-weight:600;min-width:140px}
.search-bar{display:flex;gap:0.75rem;margin-bottom:1.5rem;flex-wrap:wrap}
.search-bar input{flex:1;min-width:200px}
.speed{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:0.5rem 1rem;margin-top:1rem;text-align:center;color:#6e7681;font-size:0.8rem}
.speed strong{color:#3fb950}
table{width:100%;border-collapse:collapse}th,td{padding:0.75rem;text-align:left;border-bottom:1px solid #21262d}
th{color:#8b949e;font-weight:600;font-size:0.85rem}
"#;

fn nav_html() -> &'static str {
    r#"<nav class="nav"><h1>📡 StreamForge</h1>
    <a href="/">Browse</a><a href="/live">Live Now</a><a href="/schedule">Schedule</a>
    <a href="/leaderboard">Leaderboard</a><a href="/api/models">API</a></nav>"#
}

fn speed_bar(us: u128) -> String {
    format!(r#"<div class="speed">Rendered in <strong>{us}μs</strong> — Pure Rust, zero JS frameworks</div>"#)
}

fn wrap(title: &str, body: &str, us: u128) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>{title} — StreamForge</title><style>{CSS}</style></head>
        <body>{nav}{body}{speed}</body></html>"#,
        nav = nav_html(), speed = speed_bar(us)
    ))
}

fn model_card(m: &Model) -> String {
    let tier_class = match m.tier.as_str() {
        "Diamond" => "tier-diamond", "Gold" => "tier-gold", _ => "tier-silver"
    };
    format!(
        r#"<a href="/model/{id}" class="card" style="text-decoration:none;display:block">
        <div class="stream-preview"><div class="avatar">{avatar}</div>
        {live_badge}{viewer_count}</div>
        <div class="card-body">
            <h3>{display_name} <span class="{tier_class}" style="font-size:0.8rem">[{tier}]</span></h3>
            <div class="username">@{username} · {followers} followers</div>
            {stream_title}
            <div class="tags">{tags}</div>
            <div class="meta"><span>{category}</span><span>{total_streams} streams · {total_hours:.0}hrs</span></div>
        </div></a>"#,
        id = m.id, avatar = m.avatar, display_name = m.display_name, tier = m.tier,
        username = m.username, followers = format_num(m.followers),
        category = m.category, total_streams = m.total_streams, total_hours = m.total_hours,
        live_badge = if m.is_live { r#"<span class="live-badge">LIVE</span>"# } else { "" },
        viewer_count = if m.is_live { format!(r#"<span class="viewer-count">👁 {}</span>"#, format_num(m.viewers)) } else { String::new() },
        stream_title = if m.is_live { format!(r#"<div class="stream-title">{}</div>"#, m.stream_title) } else { String::new() },
        tags = m.tags.iter().map(|t| format!(r#"<span class="tag">#{t}</span>"#)).collect::<String>(),
    )
}

fn format_num(n: u32) -> String {
    if n >= 1000 { format!("{:.1}K", n as f64 / 1000.0) } else { n.to_string() }
}

#[derive(Deserialize)]
struct BrowseQuery {
    q: Option<String>,
    category: Option<String>,
}

async fn browse(State(state): State<Arc<AppState>>, Query(query): Query<BrowseQuery>) -> Html<String> {
    let start = std::time::Instant::now();
    let models = state.models.read().await;

    let mut filtered: Vec<&Model> = models.iter().filter(|m| {
        let q_match = query.q.as_ref().map(|q| {
            let q = q.to_lowercase();
            m.display_name.to_lowercase().contains(&q) || m.username.to_lowercase().contains(&q)
                || m.bio.to_lowercase().contains(&q) || m.tags.iter().any(|t| t.contains(&q))
        }).unwrap_or(true);
        let cat_match = query.category.as_ref().map(|c| c == &m.category).unwrap_or(true);
        q_match && cat_match
    }).collect();
    filtered.sort_by(|a, b| b.is_live.cmp(&a.is_live).then(b.viewers.cmp(&a.viewers)).then(b.followers.cmp(&a.followers)));

    let live_count = models.iter().filter(|m| m.is_live).count();
    let total_viewers: u32 = models.iter().filter(|m| m.is_live).map(|m| m.viewers).sum();
    let total_tips = *state.total_tips.read().await;

    let categories: Vec<String> = {
        let mut c: Vec<_> = models.iter().map(|m| m.category.clone()).collect();
        c.sort(); c.dedup(); c
    };

    let mut body = format!(
        r#"<div class="container">
        <div class="stats-row">
            <div class="stat-card"><div class="number" style="color:#da3633">{live_count}</div><div class="label">Live Now</div></div>
            <div class="stat-card"><div class="number">{}</div><div class="label">Total Viewers</div></div>
            <div class="stat-card"><div class="number">{}</div><div class="label">Creators</div></div>
            <div class="stat-card"><div class="number" style="color:#8b5cf6">${total_tips:.0}</div><div class="label">Tips Today</div></div>
        </div>
        <form class="search-bar" method="GET" action="/">
            <input type="text" name="q" placeholder="Search creators, tags..." value="{q}">
            <select name="category"><option value="">All Categories</option>{cat_opts}</select>
            <button class="btn" type="submit">Search</button>
        </form>
        <div class="grid">"#,
        format_num(total_viewers), models.len(),
        q = query.q.as_deref().unwrap_or(""),
        cat_opts = categories.iter().map(|c| format!(r#"<option value="{c}">{c}</option>"#)).collect::<String>(),
    );

    for m in &filtered {
        body.push_str(&model_card(m));
    }
    body.push_str("</div></div>");

    wrap("Browse", &body, start.elapsed().as_micros())
}

async fn live_now(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let models = state.models.read().await;
    let mut live: Vec<&Model> = models.iter().filter(|m| m.is_live).collect();
    live.sort_by(|a, b| b.viewers.cmp(&a.viewers));

    let mut body = String::from(r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">🔴 Live Now</h1><div class="grid">"#);
    for m in &live { body.push_str(&model_card(m)); }
    body.push_str("</div></div>");

    wrap("Live Now", &body, start.elapsed().as_micros())
}

async fn model_profile(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let models = state.models.read().await;
    let chats = state.chats.read().await;

    match models.iter().find(|m| m.id == id) {
        Some(m) => {
            let tier_class = match m.tier.as_str() { "Diamond" => "tier-diamond", "Gold" => "tier-gold", _ => "tier-silver" };
            let chat_msgs = chats.get(&id).cloned().unwrap_or_default();

            let chat_html: String = chat_msgs.iter().rev().take(50).map(|msg| {
                if msg.is_tip {
                    format!(r#"<div class="chat-msg tip"><span class="name">{}</span> <span class="tip-amount">${:.2} tip!</span><div class="text">{}</div><div style="color:#6e7681;font-size:0.75rem">{}</div></div>"#,
                        msg.username, msg.tip_amount.unwrap_or(0.0), msg.message, msg.timestamp)
                } else {
                    format!(r#"<div class="chat-msg"><span class="name">{}</span><div class="text">{}</div><div style="color:#6e7681;font-size:0.75rem">{}</div></div>"#,
                        msg.username, msg.message, msg.timestamp)
                }
            }).collect();

            let body = format!(
                r#"<div class="container"><div style="display:grid;grid-template-columns:2fr 1fr;gap:2rem">
                <div>
                    <div class="card" style="margin-bottom:1.5rem">
                        <div class="stream-preview" style="padding:3rem"><div class="avatar" style="font-size:6rem">{avatar}</div>
                        {live_badge}</div>
                        <div class="card-body">
                            <h1 style="color:#f0f6fc">{name} <span class="{tier_class}">[{tier}]</span></h1>
                            <div class="username" style="font-size:1rem">@{username}</div>
                            {stream_title}
                            <p style="margin:1rem 0;line-height:1.6">{bio}</p>
                            <div class="tags" style="margin:1rem 0">{tags}</div>
                            <div class="stats-row" style="margin-top:1rem">
                                <div class="stat-card"><div class="number">{followers}</div><div class="label">Followers</div></div>
                                <div class="stat-card"><div class="number">{streams}</div><div class="label">Streams</div></div>
                                <div class="stat-card"><div class="number">{hours:.0}</div><div class="label">Hours</div></div>
                                <div class="stat-card"><div class="number">{viewers}</div><div class="label">Watching</div></div>
                            </div>
                            <div style="display:flex;gap:0.75rem;margin-top:1rem">
                                <button class="btn" style="flex:1">Follow</button>
                                <form method="POST" action="/model/{id}/tip" style="flex:1"><button class="btn btn-tip" style="width:100%">💜 Send Tip</button></form>
                            </div>
                        </div>
                    </div>
                </div>
                <div>
                    <h3 style="color:#f0f6fc;margin-bottom:0.75rem">Chat ({msg_count})</h3>
                    <div class="chat-box">{chat_html}</div>
                    <form method="POST" action="/model/{id}/chat" style="margin-top:0.75rem;display:flex;gap:0.5rem">
                        <input type="text" name="message" placeholder="Send a message..." style="flex:1">
                        <button class="btn btn-outline">Send</button>
                    </form>
                </div></div></div>"#,
                avatar = m.avatar, name = m.display_name, tier = m.tier, username = m.username,
                bio = m.bio, id = m.id, followers = format_num(m.followers),
                streams = m.total_streams, hours = m.total_hours, viewers = format_num(m.viewers),
                live_badge = if m.is_live { format!(r#"<span class="live-badge" style="font-size:1rem;padding:6px 16px">🔴 LIVE — {}</span>"#, m.stream_title) } else { String::new() },
                stream_title = if m.is_live { format!(r#"<h2 style="margin:0.75rem 0;color:#f0f6fc">{}</h2>"#, m.stream_title) } else { String::new() },
                tags = m.tags.iter().map(|t| format!(r#"<span class="tag">#{t}</span>"#)).collect::<String>(),
                msg_count = chat_msgs.len(),
            );
            wrap(&m.display_name, &body, start.elapsed().as_micros()).into_response()
        }
        None => (StatusCode::NOT_FOUND, Html("<h1>Creator not found</h1>".to_string())).into_response(),
    }
}

#[derive(Deserialize)]
struct ChatForm {
    message: String,
}

async fn send_chat(State(state): State<Arc<AppState>>, Path(id): Path<u32>, axum::Form(form): axum::Form<ChatForm>) -> Redirect {
    let mut chats = state.chats.write().await;
    let msgs = chats.entry(id).or_default();
    msgs.push(ChatMessage {
        username: "viewer_42".into(),
        message: form.message,
        timestamp: chrono_now(),
        is_tip: false,
        tip_amount: None,
    });
    Redirect::to(&format!("/model/{id}"))
}

async fn send_tip(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Redirect {
    let amount = 5.0;
    let mut chats = state.chats.write().await;
    let msgs = chats.entry(id).or_default();
    msgs.push(ChatMessage {
        username: "viewer_42".into(),
        message: "Keep up the great work!".into(),
        timestamp: chrono_now(),
        is_tip: true,
        tip_amount: Some(amount),
    });
    let mut tips = state.total_tips.write().await;
    *tips += amount;
    Redirect::to(&format!("/model/{id}"))
}

async fn schedule_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let schedule = state.schedule.read().await;
    let models = state.models.read().await;

    let mut body = String::from(r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">📅 Upcoming Streams</h1><div class="card" style="padding:1.5rem">"#);

    for s in schedule.iter() {
        let model_name = models.iter().find(|m| m.id == s.model_id).map(|m| format!("{} {}", m.avatar, m.display_name)).unwrap_or_default();
        body.push_str(&format!(
            r#"<div class="schedule-item"><span class="schedule-time">{time}</span>
            <div><strong><a href="/model/{mid}">{model_name}</a></strong>
            <div style="color:#c9d1d9">{title}</div>
            <div style="color:#6e7681;font-size:0.8rem">{cat} · {dur:.1}hrs</div></div></div>"#,
            time = &s.scheduled_time[..16], mid = s.model_id, title = s.title,
            cat = s.category, dur = s.duration_hours,
        ));
    }
    body.push_str("</div></div>");

    wrap("Schedule", &body, start.elapsed().as_micros())
}

async fn leaderboard(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let models = state.models.read().await;
    let mut sorted: Vec<&Model> = models.iter().collect();
    sorted.sort_by(|a, b| b.followers.cmp(&a.followers));

    let mut body = String::from(r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">🏆 Leaderboard</h1>
    <table><thead><tr><th>#</th><th>Creator</th><th>Followers</th><th>Streams</th><th>Hours</th><th>Tier</th><th>Status</th></tr></thead><tbody>"#);

    for (i, m) in sorted.iter().enumerate() {
        let tier_class = match m.tier.as_str() { "Diamond" => "tier-diamond", "Gold" => "tier-gold", _ => "tier-silver" };
        let status = if m.is_live { format!(r#"<span style="color:#da3633">🔴 LIVE ({})</span>"#, format_num(m.viewers)) } else { "<span style=\"color:#6e7681\">Offline</span>".into() };
        body.push_str(&format!(
            r#"<tr><td>{rank}</td><td>{avatar} <a href="/model/{id}">{name}</a></td>
            <td>{followers}</td><td>{streams}</td><td>{hours:.0}</td>
            <td class="{tier_class}">{tier}</td><td>{status}</td></tr>"#,
            rank = i + 1, avatar = m.avatar, id = m.id, name = m.display_name,
            followers = format_num(m.followers), streams = m.total_streams,
            hours = m.total_hours, tier = m.tier,
        ));
    }
    body.push_str("</tbody></table></div>");

    wrap("Leaderboard", &body, start.elapsed().as_micros())
}

async fn api_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let models = state.models.read().await;
    let data: Vec<Value> = models.iter().map(|m| json!({
        "id": m.id, "username": m.username, "display_name": m.display_name,
        "category": m.category, "followers": m.followers, "is_live": m.is_live,
        "viewers": m.viewers, "tier": m.tier, "tags": m.tags,
    })).collect();
    let elapsed = start.elapsed();
    axum::Json(json!({ "models": data, "count": data.len(), "latency_us": elapsed.as_micros() }))
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("2026-02-08T{h:02}:{m:02}:{s:02}Z")
}

#[tokio::main]
async fn main() {
    let models = seed_models();
    let live_count = models.iter().filter(|m| m.is_live).count();
    eprintln!("livestream ready: {} creators, {} live now", models.len(), live_count);

    let state = Arc::new(AppState {
        models: RwLock::new(models),
        chats: RwLock::new(HashMap::new()),
        schedule: RwLock::new(seed_schedule()),
        total_tips: RwLock::new(0.0),
    });

    let app = Router::new()
        .route("/", get(browse))
        .route("/live", get(live_now))
        .route("/model/{id}", get(model_profile))
        .route("/model/{id}/chat", post(send_chat))
        .route("/model/{id}/tip", post(send_tip))
        .route("/schedule", get(schedule_page))
        .route("/leaderboard", get(leaderboard))
        .route("/api/models", get(api_models))
        .with_state(state);

    let addr = "0.0.0.0:4002";
    eprintln!("livestream listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
