pub mod app;
pub mod codie;
pub mod consent_gate;
pub mod cookie_jar;
pub mod emoji_rewriter;
pub mod genesis_shield;
pub mod gentlyos;
pub mod glyph_registry;
pub mod health;
pub mod inbox;
pub mod models;
pub mod pages;
pub mod projects;
pub mod semantic_chars;
pub mod sessions;
pub mod staging;
pub mod surface;
pub mod tier;
pub mod tools;
pub mod tos_interceptor;

/// Check if the request comes from HTMX (has HX-Request header).
pub fn is_htmx(headers: &axum::http::HeaderMap) -> bool {
    headers.get("HX-Request").is_some()
}

/// HTML-escape a string to prevent XSS in hand-built HTML responses.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Wrap fragment HTML in the base page shell for direct URL access.
/// Mirrors base.html but allows embedding pre-rendered fragment content.
pub fn wrap_page(title: &str, content: &str) -> String {
    let base = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>cage-web | __TITLE__</title>
    <link rel="stylesheet" href="/static/style.css">
    <script src="/static/htmx.min.js"></script>
    <script src="/static/sse.js"></script>
</head>
<body>
    <nav class="sidebar">
        <div class="logo">
            <pre class="logo-art"> &#9614;&#9627;&#9611;&#9611;&#9611;&#9620;&#9612;
&#9629;&#9620;&#9611;&#9611;&#9611;&#9611;&#9611;&#9627;&#9624;
  &#9624;&#9624; &#9629;&#9629;</pre>
            <span class="logo-text">cage-web</span>
        </div>
        <ul class="nav-links">
            <li><a href="/" hx-get="/" hx-target="#main" hx-push-url="true">Dashboard</a></li>
            <li><a href="/sessions" hx-get="/sessions" hx-target="#main" hx-push-url="true">Sessions</a></li>
            <li><a href="/tree" hx-get="/tree" hx-target="#main" hx-push-url="true">GentlyOS Tree</a></li>
            <li><a href="/codie" hx-get="/codie" hx-target="#main" hx-push-url="true">CODIE Programs</a></li>
            <li><a href="/tier" hx-get="/tier" hx-target="#main" hx-push-url="true">Tier Hierarchy</a></li>
            <li><a href="/surface" hx-get="/surface" hx-target="#main" hx-push-url="true">IO Surface</a></li>
            <li class="nav-section">The Field</li>
            <li><a href="/staging" hx-get="/staging" hx-target="#main" hx-push-url="true">Staging</a></li>
            <li class="nav-section">Intelligence</li>
            <li><a href="/models" hx-get="/models" hx-target="#main" hx-push-url="true">Models</a></li>
            <li><a href="/tools" hx-get="/tools" hx-target="#main" hx-push-url="true">Tools</a></li>
            <li><a href="/projects" hx-get="/projects" hx-target="#main" hx-push-url="true">Projects</a></li>
            <li class="nav-section">IO Tools</li>
            <li><a href="/cookie-jar" hx-get="/cookie-jar" hx-target="#main" hx-push-url="true">Cookie Jar</a></li>
            <li><a href="/glyph-registry" hx-get="/glyph-registry" hx-target="#main" hx-push-url="true">Glyph Registry</a></li>
            <li><a href="/consent-gate" hx-get="/consent-gate" hx-target="#main" hx-push-url="true">Consent Gate</a></li>
            <li><a href="/genesis-shield" hx-get="/genesis-shield" hx-target="#main" hx-push-url="true">Genesis Shield</a></li>
            <li><a href="/inbox" hx-get="/inbox" hx-target="#main" hx-push-url="true">Inbox Pipeline</a></li>
            <li><a href="/emoji-rewriter" hx-get="/emoji-rewriter" hx-target="#main" hx-push-url="true">Emoji Rewriter</a></li>
            <li><a href="/semantic-chars" hx-get="/semantic-chars" hx-target="#main" hx-push-url="true">Semantic Chars</a></li>
            <li><a href="/tos-interceptor" hx-get="/tos-interceptor" hx-target="#main" hx-push-url="true">ToS Interceptor</a></li>
        </ul>
        <div class="health-bar" hx-get="/partials/health" hx-trigger="load, every 5s" hx-swap="innerHTML">
            Loading...
        </div>
    </nav>
    <main id="main">
__CONTENT__
    </main>
    <div id="spinner" class="htmx-indicator">Loading...</div>
</body>
</html>"##;
    base.replace("__TITLE__", &html_escape(title))
        .replace("__CONTENT__", content)
}
