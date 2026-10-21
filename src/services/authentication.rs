/// Service to manage user authentication
///
/// Authentication is managed with idconnect protocole.
pub struct AuthenticationService;

impl AuthenticationService {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for AuthenticationService {
    fn default() -> Self {
        Self::new()
    }
}
