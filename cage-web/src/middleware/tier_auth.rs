//! Tier Auth Middleware -- extracts layer/tier from requests.
//!
//! Resolution order:
//!   1. X-Gently-Tier header
//!   2. gently_tier cookie
//!   3. ?tier= query parameter
//!   4. Default: User (L5)
//!
//! The resolved Layer is injected into request extensions so handlers
//! can call `req.extensions().get::<Layer>()`.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Dashboard visibility layer. L0 = highest privilege, L5 = lowest.
/// Mirrors gently-core::layer::Layer but kept local to avoid workspace coupling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Layer {
    Admin     = 0,  // L0 -- Tom only
    GentlyDev = 1,  // L1 -- internal team
    DevLevel  = 2,  // L2 -- public SDK
    OsAdmin   = 3,  // L3 -- tenant sysadmin
    RootUser  = 4,  // L4 -- dashboard owner
    User      = 5,  // L5 -- end user
}

impl Layer {
    pub fn from_tier(tier: &str) -> Self {
        match tier {
            "admin" => Layer::Admin,
            "founder" => Layer::Admin,
            "dev" => Layer::DevLevel,
            "pro" => Layer::OsAdmin,
            "basic" => Layer::RootUser,
            "free" => Layer::User,
            _ => Layer::User,
        }
    }

    pub fn level(self) -> u8 {
        self as u8
    }

    pub fn has_access(self, required: Layer) -> bool {
        self.level() <= required.level()
    }

    pub fn label(self) -> &'static str {
        match self {
            Layer::Admin => "Admin",
            Layer::GentlyDev => "GentlyDev",
            Layer::DevLevel => "DevLevel",
            Layer::OsAdmin => "OsAdmin",
            Layer::RootUser => "RootUser",
            Layer::User => "User",
        }
    }

    pub fn tier_name(self) -> &'static str {
        match self {
            Layer::Admin => "founder",
            Layer::GentlyDev => "dev",
            Layer::DevLevel => "dev",
            Layer::OsAdmin => "pro",
            Layer::RootUser => "basic",
            Layer::User => "free",
        }
    }

    pub fn badge_class(self) -> &'static str {
        match self {
            Layer::Admin => "tier-founder",
            Layer::GentlyDev => "tier-dev",
            Layer::DevLevel => "tier-dev",
            Layer::OsAdmin => "tier-pro",
            Layer::RootUser => "tier-basic",
            Layer::User => "tier-free",
        }
    }
}

/// Axum middleware function that resolves the tier and injects Layer.
pub async fn tier_auth(mut req: Request, next: Next) -> Response {
    let tier = extract_tier(&req);
    let layer = Layer::from_tier(&tier);
    req.extensions_mut().insert(layer);
    next.run(req).await
}

fn extract_tier(req: &Request) -> String {
    // 1. X-Gently-Tier header
    if let Some(val) = req.headers().get("X-Gently-Tier") {
        if let Ok(s) = val.to_str() {
            return s.to_lowercase();
        }
    }

    // 2. gently_tier cookie
    if let Some(cookie_header) = req.headers().get("Cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            for pair in cookies.split(';') {
                let pair = pair.trim();
                if let Some(val) = pair.strip_prefix("gently_tier=") {
                    return val.to_lowercase();
                }
            }
        }
    }

    // 3. ?tier= query parameter
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(val) = pair.strip_prefix("tier=") {
                return val.to_lowercase();
            }
        }
    }

    // 4. Default
    "free".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_from_tier_founder() {
        assert_eq!(Layer::from_tier("founder"), Layer::Admin);
    }

    #[test]
    fn layer_from_tier_unknown() {
        assert_eq!(Layer::from_tier("xyz"), Layer::User);
    }

    #[test]
    fn layer_access_check() {
        assert!(Layer::Admin.has_access(Layer::User));
        assert!(!Layer::User.has_access(Layer::Admin));
        assert!(Layer::OsAdmin.has_access(Layer::OsAdmin));
    }

    #[test]
    fn badge_classes() {
        assert_eq!(Layer::Admin.badge_class(), "tier-founder");
        assert_eq!(Layer::User.badge_class(), "tier-free");
        assert_eq!(Layer::OsAdmin.badge_class(), "tier-pro");
    }
}
