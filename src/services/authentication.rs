use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::{
    model::{
        auth::{OAuthToken, find_field_in_blob},
        config::Configuration,
    },
    services::database::users::OAuthLoginError,
    utils::token::generate_token,
};
use serde::Deserialize;

/// Define the OAuth callback information
#[derive(Debug, Deserialize)]
pub struct OAuthLoginParams {
    code: String,
    state: String,
}

/// User information from OIDC
#[derive(Debug, Default)]
pub struct UserInfo {
    pub email: String,
    pub full_name: Option<String>,
}

/// Service to manage user authentication
///
/// Authentication is managed with idconnect protocole.
pub struct AuthenticationService {
    http_client: Client,
    pending_client_connect: Cache<String, String>,
}

impl AuthenticationService {
    #[must_use]
    pub fn new() -> Self {
        let http_client = Client::new();
        let pending_client_connect = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_mins(5))
            .build();
        Self {
            http_client,
            pending_client_connect,
        }
    }

    /// Gets the redirection response when not authenticated
    #[must_use]
    pub async fn get_auth_redirect(&self, configuration: &Configuration) -> String {
        // redirect to login
        let nonce = generate_token(64);
        let oauth_state = generate_token(32);
        self.pending_client_connect.insert(oauth_state.clone(), nonce.clone()).await;
        format!(
            "{}?response_type={}&redirect_uri={}&client_id={}&scope={}&nonce={}&state={}",
            configuration.oauth_login_uri,
            "code",
            urlencoding::encode(&format!("{}/webapp/oauthcallback.html", configuration.web_public_uri)),
            urlencoding::encode(&configuration.oauth_client_id),
            urlencoding::encode(&configuration.oauth_client_scope),
            nonce,
            oauth_state
        )
    }

    /// Attempts to login using an OAuth code
    pub async fn login_with_oauth_code(
        &self,
        configuration: &Configuration,
        params: OAuthLoginParams,
    ) -> Result<UserInfo, OAuthLoginError> {
        let OAuthLoginParams { code, state } = params;
        let nonce = self
            .pending_client_connect
            .get(&state)
            .await
            .ok_or(OAuthLoginError::NoCorrespondingCode)?;

        // retrieve the token
        let response = self
            .http_client
            .post(&configuration.oauth_token_uri)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &configuration.oauth_callback_uri),
                ("client_id", &configuration.oauth_client_id),
                ("client_secret", &configuration.oauth_client_secret),
            ])
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|source| OAuthLoginError::GetRetrieveToken {
                source,
                oauth_token_uri: configuration.oauth_token_uri.as_str().into(),
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.bytes().await.unwrap();
            return Err(OAuthLoginError::RetrieveTokenFailed {
                status,
                body: String::from_utf8_lossy(&body).into(),
            });
        }
        let body = response
            .bytes()
            .await
            .map_err(|source| OAuthLoginError::RetrieveTokenResponse { source })?;
        let token =
            serde_json::from_slice::<OAuthToken>(&body).map_err(|source| OAuthLoginError::ParseRetrieveTokenResponse {
                source,
                body: String::from_utf8_lossy(&body).into(),
            })?;

        // retrieve the user profile
        let response = self
            .http_client
            .get(&configuration.oauth_userinfo_uri)
            .header("authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .map_err(|source| OAuthLoginError::GetUserProfile {
                source,
                oauth_userinfo_uri: configuration.oauth_userinfo_uri.as_str().into(),
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(OAuthLoginError::RetrieveUserProfile {
                status,
                uri: configuration.oauth_userinfo_uri.as_str().into(),
            });
        }
        let body = response
            .bytes()
            .await
            .map_err(|source| OAuthLoginError::GetUserProfileResponse { source })?;
        let auth_response = serde_json::from_slice::<serde_json::Value>(&body).map_err(|source| {
            OAuthLoginError::ParseGetUserProfileResponse {
                source,
                body: String::from_utf8_lossy(&body).into(),
            }
        })?;

        let resp_nonce = find_field_in_blob(&auth_response, "nonce").ok_or(OAuthLoginError::NonceMissingInResponse)?;
        if resp_nonce != nonce {
            return Err(OAuthLoginError::InvalidNonceInResponse);
        }

        let email = find_field_in_blob(&auth_response, &configuration.oauth_userinfo_path_email)
            .ok_or_else(|| OAuthLoginError::EmailMissingInUserInfo(auth_response.to_string()))?;
        let full_name =
            find_field_in_blob(&auth_response, &configuration.oauth_userinfo_path_fullname).map(ToString::to_string);

        Ok(UserInfo {
            email: email.to_string(),
            full_name,
        })
    }
}

impl Default for AuthenticationService {
    fn default() -> Self {
        Self::new()
    }
}
