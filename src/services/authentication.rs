use std::time::Duration;

use moka::future::Cache;
use openidconnect::{
    ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::{Client, ClientBuilder, Url, redirect},
};

use crate::{
    model::{
        auth::{OAuthToken, find_field_in_blob},
        config::Configuration,
    },
    services::database::users::OAuthLoginError,
};
use serde::Deserialize;

// Type returned from ProviderMetadata discovering
type OIDCProviderMetadata = openidconnect::ProviderMetadata<
    openidconnect::EmptyAdditionalProviderMetadata,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreClientAuthMethod,
    openidconnect::core::CoreClaimName,
    openidconnect::core::CoreClaimType,
    openidconnect::core::CoreGrantType,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJweKeyManagementAlgorithm,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreResponseMode,
    openidconnect::core::CoreResponseType,
    openidconnect::core::CoreSubjectIdentifierType,
>;

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
    pending_client_connect: Cache<String, Nonce>,
}

impl AuthenticationService {
    #[must_use]
    pub fn new() -> Self {
        let http_client = ClientBuilder::new()
            .redirect(redirect::Policy::none())
            .build()
            .expect("reqwest Client should build");

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
    pub async fn get_auth_redirect(&self, configuration: &Configuration) -> Url {
        let provider_metadata = self
            .provider_metadata(configuration)
            .await
            .expect("failed to construct oidc provider metadata");
        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(configuration.oauth_client_id.clone()),
            Some(ClientSecret::new(configuration.oauth_client_secret.clone())),
        )
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/webapp/oauthcallback.html", configuration.web_public_uri))
                .expect("redirect url failed"),
        );

        // Generate a PKCE challenge.
        //let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, csrf_token, nonce) = oidc_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            // Set the desired scopes.
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            // Set the PKCE code challenge.
            //.set_pkce_challenge(pkce_challenge)
            .url();

        self.pending_client_connect.insert(csrf_token.into_secret(), nonce).await;
        auth_url
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

        // // retrieve the token
        // // Now you can exchange it for an access token and ID token.
        // let token_response = client
        //     .exchange_code(AuthorizationCode::new("some authorization code".to_string()))?
        //     // Set the PKCE code verifier.
        //     .set_pkce_verifier(pkce_verifier)
        //     .request(&http_client)?;
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
        if resp_nonce != nonce.secret() {
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

    async fn provider_metadata(&self, configuration: &Configuration) -> Result<OIDCProviderMetadata, OAuthLoginError> {
        let oidc_url = IssuerUrl::new(configuration.oauth_discovery_uri.clone()).expect("url construct failed");
        CoreProviderMetadata::discover_async(oidc_url, &self.http_client)
            .await
            .map_err(|err| OAuthLoginError::DiscoveryFailed {
                err,
                uri: configuration.oauth_discovery_uri.clone(),
            })
    }
}

impl Default for AuthenticationService {
    fn default() -> Self {
        Self::new()
    }
}
