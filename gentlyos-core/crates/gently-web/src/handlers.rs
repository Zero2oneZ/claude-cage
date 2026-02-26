//! Route handlers for the web GUI

use axum::{
    extract::{Form, State},
    http::{header, StatusCode, Request},
    response::{Html, IntoResponse, Response, Redirect},
    Json,
    middleware::Next,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_cookies::{Cookie, Cookies};

use crate::state::{AppState, ChatMessage, UserSession};
use crate::templates;

// ============== Authentication Middleware ==============

use axum::body::Body;

/// Extract session from cookie
pub fn get_session_from_cookies(cookies: &Cookies, state: &AppState) -> Option<UserSession> {
    cookies
        .get("gently_session")
        .and_then(|c| state.auth.validate_token(c.value()))
}

/// Authentication middleware - redirects to login if not authenticated
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    request: Request<Body>,
    next: Next,
) -> Response {
    if get_session_from_cookies(&cookies, &state).is_some() {
        next.run(request).await
    } else {
        Redirect::to("/login").into_response()
    }
}

/// Premium authentication middleware - requires premium subscription
pub async fn require_premium(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Some(session) = get_session_from_cookies(&cookies, &state) {
        if session.is_premium {
            return next.run(request).await;
        }
        // Not premium - show upgrade message
        return Html(templates::upgrade_required_html()).into_response();
    }
    Redirect::to("/login").into_response()
}

// ============== Auth Page Handlers ==============

/// Login page
pub async fn login_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let csrf = state.auth.generate_csrf();
    Html(templates::login_html(&csrf))
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
    pub csrf_token: String,
}

/// Handle login form submission
pub async fn login_submit(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    Form(input): Form<LoginInput>,
) -> impl IntoResponse {
    // Validate CSRF
    if !state.auth.validate_csrf(&input.csrf_token) {
        return (
            StatusCode::BAD_REQUEST,
            Html(templates::login_error_html("Invalid request. Please try again.")),
        ).into_response();
    }

    // Attempt login
    if let Some(session) = state.auth.login(&input.username, &input.password) {
        // Set session cookie
        let mut cookie = Cookie::new("gently_session", session.token);
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(false); // Set to true in production with HTTPS
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
        cookies.add(cookie);

        // Log security event
        {
            let mut events = state.security_events.write().unwrap();
            events.push(crate::state::SecurityEvent::new(
                "login",
                "info",
                &format!("User '{}' logged in", input.username),
            ));
        }

        Redirect::to("/scene").into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Html(templates::login_error_html("Invalid username or password")),
        ).into_response()
    }
}

/// Handle logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    if let Some(cookie) = cookies.get("gently_session") {
        state.auth.logout(cookie.value());
        cookies.remove(Cookie::from("gently_session"));
    }
    Redirect::to("/login")
}

// ============== Page Handlers ==============

/// Main index page - redirects to scene
pub async fn index() -> impl IntoResponse {
    Html(templates::index_html())
}

/// The ONE SCENE - main adaptive interface
pub async fn scene(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::scene_html(&state))
}

// ============== HTMX Partial Handlers ==============

/// Chat panel partial
pub async fn chat_panel(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let history = state.chat_history.read().unwrap();
    Html(templates::chat_panel_html(&history))
}

#[derive(Deserialize)]
pub struct ChatInput {
    pub message: String,
}

/// Send chat message
pub async fn chat_send(
    State(state): State<Arc<AppState>>,
    Form(input): Form<ChatInput>,
) -> impl IntoResponse {
    // Add user message with auto-persist
    state.add_chat_message(ChatMessage::user(&input.message));

    // Generate response (placeholder - would call gently-brain)
    let response = format!(
        "I received your message: \"{}\". This is a placeholder response from GentlyOS.",
        input.message
    );

    // Add assistant response with auto-persist
    state.add_chat_message(ChatMessage::assistant(&response, Some(42)));

    // Return updated chat panel
    let history = state.chat_history.read().unwrap();
    Html(templates::chat_panel_html(&history))
}

/// Clear chat history
pub async fn chat_clear(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    state.clear_chat_history();
    Html(templates::chat_panel_html(&[]))
}

/// Feed panel partial
pub async fn feed_panel(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let feed = state.feed.read().unwrap();
    Html(templates::feed_panel_html(&feed))
}

#[derive(Deserialize)]
pub struct BoostInput {
    pub name: String,
    pub amount: Option<f32>,
}

/// Boost a feed item
pub async fn feed_boost(
    State(state): State<Arc<AppState>>,
    Form(input): Form<BoostInput>,
) -> impl IntoResponse {
    let amount = input.amount.unwrap_or(0.3);
    {
        let mut feed = state.feed.write().unwrap();
        feed.boost(&input.name, amount);
    }

    let feed = state.feed.read().unwrap();
    Html(templates::feed_panel_html(&feed))
}

/// Security panel partial
pub async fn security_panel(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let events = state.security_events.read().unwrap();
    Html(templates::security_panel_html(&events, &state))
}

/// Search panel partial
pub async fn search_panel(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::search_panel_html(&[]))
}

#[derive(Deserialize)]
pub struct SearchInput {
    pub query: String,
}

/// Execute search query
pub async fn search_query(
    State(state): State<Arc<AppState>>,
    Form(input): Form<SearchInput>,
) -> impl IntoResponse {
    use gently_search::ContextRouter;

    let index = state.index.read().unwrap();
    let feed = state.feed.read().unwrap();

    let router = ContextRouter::new().with_max_results(10);
    let results = router.search(&input.query, &index, Some(&feed));

    Html(templates::search_results_html(&results))
}

/// Status panel partial
pub async fn status_panel(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::status_panel_html(&state))
}

// ============== API Handlers ==============

/// Health check
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "gently-web",
        "version": "1.0.0"
    }))
}

/// Full status API
pub async fn api_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let feed = state.feed.read().unwrap();
    let index = state.index.read().unwrap();

    Json(serde_json::json!({
        "uptime_secs": state.uptime_secs(),
        "feed": {
            "total_items": feed.items().len(),
            "hot_items": feed.hot_items().len(),
            "active_items": feed.active_items().len()
        },
        "index": {
            "stats": index.stats().to_string()
        },
        "chat_messages": state.chat_history.read().unwrap().len(),
        "security_events": state.security_events.read().unwrap().len()
    }))
}

#[derive(Deserialize)]
pub struct ApiChatRequest {
    pub message: String,
    pub history: Option<Vec<ChatMessage>>,
}

#[derive(Serialize)]
pub struct ApiChatResponse {
    pub response: String,
    pub tokens_used: u32,
}

/// Chat API endpoint
pub async fn api_chat(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ApiChatRequest>,
) -> impl IntoResponse {
    // Placeholder response
    Json(ApiChatResponse {
        response: format!("Response to: {}", req.message),
        tokens_used: 42,
    })
}

#[derive(Deserialize)]
pub struct ApiSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

/// Search API endpoint
pub async fn api_search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ApiSearchRequest>,
) -> impl IntoResponse {
    use gently_search::ContextRouter;

    let index = state.index.read().unwrap();
    let feed = state.feed.read().unwrap();

    let router = ContextRouter::new().with_max_results(req.limit.unwrap_or(10));
    let results = router.search(&req.query, &index, Some(&feed));

    let results_json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.thought.id.to_string(),
                "content": r.thought.content,
                "score": r.score,
                "domain": r.thought.shape.domain
            })
        })
        .collect();

    Json(serde_json::json!({
        "query": req.query,
        "count": results.len(),
        "results": results_json
    }))
}

// ============== Static Assets ==============

/// CSS stylesheet
pub async fn style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        templates::STYLE_CSS,
    )
}

/// HTMX JavaScript (embedded minimal version)
pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("static/htmx.min.js"),
    )
}

// ============== Alexandria Premium Handlers ==============

/// Alexandria main panel
pub async fn alexandria_panel(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::alexandria_panel_html())
}

/// Alexandria graph visualization
pub async fn alexandria_graph(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Get concepts from ThoughtIndex
    let index = state.index.read().unwrap();
    let thoughts = index.thoughts();

    // Build concept list with scores (use access_count as score proxy)
    let concepts: Vec<(String, f32)> = thoughts
        .iter()
        .take(15) // Limit for visualization
        .map(|t| (t.content.chars().take(20).collect::<String>(), t.access_count as f32 / 10.0))
        .collect();

    // Generate some demo edges based on domain similarity
    let edges: Vec<(usize, usize)> = (0..concepts.len())
        .flat_map(|i| {
            (i+1..concepts.len())
                .filter(move |j| (i + j) % 3 == 0) // Demo connectivity
                .map(move |j| (i, j))
        })
        .take(10)
        .collect();

    Html(templates::alexandria_graph_html(&concepts, &edges))
}

/// BBBCP query panel
pub async fn alexandria_bbbcp(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::bbbcp_panel_html())
}

#[derive(Deserialize)]
pub struct BbbcpInput {
    pub bone: Option<String>,
    pub circle: Option<String>,
    pub blob: Option<String>,
}

/// Execute BBBCP query
pub async fn alexandria_bbbcp_query(
    State(state): State<Arc<AppState>>,
    Form(input): Form<BbbcpInput>,
) -> impl IntoResponse {
    use gently_search::ContextRouter;

    let blob_query = input.blob.unwrap_or_default();
    let bone_constraints: Vec<&str> = input.bone.as_deref()
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).collect())
        .unwrap_or_default();
    let circle_eliminations: Vec<&str> = input.circle.as_deref()
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).collect())
        .unwrap_or_default();

    // Search with constraints
    let index = state.index.read().unwrap();
    let feed = state.feed.read().unwrap();
    let router = ContextRouter::new().with_max_results(10);
    let results = router.search(&blob_query, &index, Some(&feed));

    // Calculate elimination ratio
    let total_thoughts = index.thoughts().len().max(1);
    let remaining = results.len();
    let elimination_ratio = 1.0 - (remaining as f32 / total_thoughts as f32);

    // Build result summary
    let result = if results.is_empty() {
        "No results found. Try adjusting your constraints.".to_string()
    } else {
        let mut summary = format!("Found {} results matching your query.\n\n", results.len());
        if !bone_constraints.is_empty() {
            summary.push_str(&format!("Applied {} BONE constraints.\n", bone_constraints.len()));
        }
        if !circle_eliminations.is_empty() {
            summary.push_str(&format!("Applied {} CIRCLE eliminations.\n", circle_eliminations.len()));
        }
        summary.push_str("\nTop matches:\n");
        for (i, r) in results.iter().take(5).enumerate() {
            summary.push_str(&format!("{}. {} (score: {:.2})\n", i + 1, r.thought.content.chars().take(50).collect::<String>(), r.score));
        }
        summary
    };

    // Quality based on result relevance
    let quality = if results.is_empty() { 0.2 } else {
        results.iter().map(|r| r.score).sum::<f32>() / results.len().max(1) as f32
    };

    Html(templates::bbbcp_results_html(&result, quality, elimination_ratio))
}

/// Tesseract visualization
pub async fn alexandria_tesseract(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // Show default active faces
    let active = vec!["WHO", "WHAT"];
    Html(templates::tesseract_panel_html(&active))
}

/// 5W dimension panel
pub async fn alexandria_5w(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(templates::dimension_5w_panel_html())
}

#[derive(Deserialize)]
pub struct Dimension5wQuery {
    pub query: String,
}

/// Execute 5W query with dimensional collapse
pub async fn alexandria_5w_query(
    State(state): State<Arc<AppState>>,
    Form(input): Form<Dimension5wQuery>,
) -> impl IntoResponse {
    use gently_search::ContextRouter;

    let index = state.index.read().unwrap();
    let feed = state.feed.read().unwrap();
    let router = ContextRouter::new().with_max_results(10);
    let results = router.search(&input.query, &index, Some(&feed));

    // Build table from results
    let columns = vec!["WHAT", "WHERE", "WHEN"];
    let rows: Vec<Vec<String>> = results
        .iter()
        .take(10)
        .map(|r| {
            vec![
                r.thought.content.chars().take(30).collect::<String>(),
                format!("Domain {}", r.thought.shape.domain),
                r.thought.created_at.format("%Y-%m-%d").to_string(),
            ]
        })
        .collect();

    Html(templates::dimension_5w_results_html(&columns, &rows))
}

#[derive(Deserialize)]
pub struct DimensionPinInput {
    pub dim: String,
}

/// Pin a dimension
pub async fn alexandria_5w_pin(
    State(_state): State<Arc<AppState>>,
    Form(input): Form<DimensionPinInput>,
) -> impl IntoResponse {
    Html(format!(
        "<div style=\"color: var(--accent); padding: 12px; background: var(--bg-tertiary); border-radius: 8px;\">
            Pinned dimension: <strong>{}</strong>
        </div>",
        input.dim.to_uppercase()
    ))
}
