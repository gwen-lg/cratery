use std::time::Duration;

use moka::future::Cache;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse as _,
    RedirectUrl, Scope,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::{Client, ClientBuilder, Url, redirect},
};
use serde::Deserialize;

use crate::{model::config::Configuration, services::database::users::OAuthLoginError};

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
        //TODO: .set_revocation_url ?

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
        Ok(auth_url)
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

        let provider_metadata = self.provider_metadata(configuration).await?;
        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(configuration.oauth_client_id.clone()),
            Some(ClientSecret::new(configuration.oauth_client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/webapp/oauthcallback.html", configuration.web_public_uri))
                .expect("redirect url failed"),
        );

        let auth_code = AuthorizationCode::new(code);
        // Now you can exchange it for an access token and ID token.
        let token_response = oidc_client
            .exchange_code(auth_code)
            .unwrap()
            //.set_pkce_verifier(pkce_verifier) // Set the PKCE code verifier.
            .request_async(&self.http_client)
            .await
            .map_err(OAuthLoginError::RequestTokenError)?;

        // Extract the ID token claims after verifying its authenticity and nonce.
        let id_token_verifier = oidc_client.id_token_verifier();
        let id_token = token_response
            .extra_fields()
            .id_token()
            .ok_or(OAuthLoginError::MissingTokenId)?;
        let id_token_claims = id_token.claims(&id_token_verifier, &nonce).map_err(OAuthLoginError::Claims)?;

        // Verify the access token hash to ensure that the access token hasn't been substituted for another user's.
        if let Some(expected_access_token_hash) = id_token_claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg().unwrap(),
                id_token.signing_key(&id_token_verifier).unwrap(),
            )
            .unwrap();
            if actual_access_token_hash != *expected_access_token_hash {
                return Err(OAuthLoginError::InvalidAccessToken);
            }
        }

        // The authenticated user's identity is now available. See the IdTokenClaims struct for a
        // complete listing of the available claims.
        log::debug!(
            "User {} with e-mail address {} has authenticated successfully",
            id_token_claims.subject().as_str(),
            id_token_claims.email().map_or("<not provided>", |email| email.as_str()),
        );

        let email = id_token_claims.email().ok_or_else(|| {
            let response = format!("{token_response:#?}");
            OAuthLoginError::EmailMissingInUserInfo(response)
        })?;
        let full_name = id_token_claims
            .name()
            .ok_or_else(|| {
                let response = format!("{token_response:#?}");
                OAuthLoginError::NameMissingInUserInfo(response)
            })?
            .get(None)
            .map(|user_name| user_name.to_string());

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
