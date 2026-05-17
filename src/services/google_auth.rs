use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::CustomError;

const GOOGLE_ISSUERS: &[&str] = &["accounts.google.com", "https://accounts.google.com"];
const JWKS_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

pub const DEFAULT_GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

struct JwksCache {
    keys: HashMap<String, Arc<DecodingKey>>,
    fetched_at: Instant,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GoogleTokenInfo {
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub picture: String,
}

pub struct GoogleTokenVerifier {
    client_id: String,
    jwks_url: String,
    http: reqwest::Client,
    cache: RwLock<Option<JwksCache>>,
}

impl GoogleTokenVerifier {
    pub fn new(client_id: String, jwks_url: String) -> Self {
        Self {
            client_id,
            jwks_url,
            http: reqwest::Client::new(),
            cache: RwLock::new(None),
        }
    }

    pub async fn verify(&self, id_token: &str) -> Result<GoogleTokenInfo, CustomError> {
        let header = decode_header(id_token)
            .map_err(|_| CustomError::UnauthorizedError("Invalid token header".into()))?;

        if header.alg != Algorithm::RS256 {
            return Err(CustomError::UnauthorizedError(
                "Unexpected token alg".into(),
            ));
        }

        let kid = header
            .kid
            .ok_or_else(|| CustomError::UnauthorizedError("Token missing kid".into()))?;

        let key = match self.key_for(&kid, false).await? {
            Some(k) => k,
            None => self
                .key_for(&kid, true)
                .await?
                .ok_or_else(|| CustomError::UnauthorizedError("Unknown token kid".into()))?,
        };

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(GOOGLE_ISSUERS);

        let data = decode::<GoogleTokenInfo>(id_token, &key, &validation)
            .map_err(|_| CustomError::UnauthorizedError("Invalid Google token".into()))?;

        if !data.claims.email_verified {
            return Err(CustomError::UnauthorizedError(
                "Google email not verified".into(),
            ));
        }

        Ok(data.claims)
    }

    async fn key_for(
        &self,
        kid: &str,
        force_refresh: bool,
    ) -> Result<Option<Arc<DecodingKey>>, CustomError> {
        if !force_refresh {
            let guard = self.cache.read().await;
            if let Some(cache) = guard.as_ref() {
                if cache.fetched_at.elapsed() < JWKS_CACHE_TTL {
                    return Ok(cache.keys.get(kid).cloned());
                }
            }
        }

        let jwks: Jwks = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|_| CustomError::InternalError("JWKS fetch failed".into()))?
            .error_for_status()
            .map_err(|_| CustomError::InternalError("JWKS fetch returned error status".into()))?
            .json()
            .await
            .map_err(|_| CustomError::InternalError("JWKS parse failed".into()))?;

        let mut keys: HashMap<String, Arc<DecodingKey>> = HashMap::new();
        for k in jwks.keys {
            if let Ok(dk) = DecodingKey::from_rsa_components(&k.n, &k.e) {
                keys.insert(k.kid, Arc::new(dk));
            }
        }
        let hit = keys.get(kid).cloned();
        *self.cache.write().await = Some(JwksCache {
            keys,
            fetched_at: Instant::now(),
        });

        Ok(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct TestKey {
        pem: Vec<u8>,
        kid: String,
        n: String,
        e: String,
    }

    fn gen_key() -> TestKey {
        let mut rng = rsa::rand_core::OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pub_key = priv_key.to_public_key();
        let pem_doc = priv_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).unwrap();
        let pem = pem_doc.as_bytes().to_vec();
        let n = URL_SAFE_NO_PAD.encode(pub_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(pub_key.e().to_bytes_be());
        TestKey {
            pem,
            kid: "test-kid".into(),
            n,
            e,
        }
    }

    fn sign(key: &TestKey, claims: serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(key.kid.clone());
        let ek = EncodingKey::from_rsa_pem(&key.pem).unwrap();
        encode(&header, &claims, &ek).unwrap()
    }

    async fn jwks_server(key: &TestKey) -> MockServer {
        let server = MockServer::start().await;
        let body = json!({
            "keys": [{
                "kid": key.kid,
                "kty": "RSA",
                "alg": "RS256",
                "use": "sig",
                "n": key.n,
                "e": key.e,
            }]
        });
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        server
    }

    fn exp_in(sec: i64) -> i64 {
        chrono::Utc::now().timestamp() + sec
    }

    fn valid_claims() -> serde_json::Value {
        json!({
            "iss": "https://accounts.google.com",
            "aud": "client-123",
            "exp": exp_in(300),
            "iat": exp_in(-1),
            "sub": "user-1",
            "email": "a@b.com",
            "email_verified": true,
            "name": "Alice",
            "picture": "https://example.com/a.png"
        })
    }

    #[tokio::test]
    async fn verifies_valid_token() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let token = sign(&key, valid_claims());

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        let info = v.verify(&token).await.expect("should verify");
        assert_eq!(info.sub, "user-1");
        assert_eq!(info.email, "a@b.com");
        assert!(info.email_verified);
    }

    #[tokio::test]
    async fn rejects_unverified_email() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let mut claims = valid_claims();
        claims["email_verified"] = json!(false);
        let token = sign(&key, claims);

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        let err = v.verify(&token).await.unwrap_err();
        assert!(matches!(err, CustomError::UnauthorizedError(_)));
    }

    #[tokio::test]
    async fn rejects_wrong_audience() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let mut claims = valid_claims();
        claims["aud"] = json!("someone-else");
        let token = sign(&key, claims);

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        assert!(v.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn rejects_wrong_issuer() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let mut claims = valid_claims();
        claims["iss"] = json!("https://evil.example.com");
        let token = sign(&key, claims);

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        assert!(v.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let mut claims = valid_claims();
        claims["exp"] = json!(exp_in(-3600));
        let token = sign(&key, claims);

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        assert!(v.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn rejects_tampered_signature() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let token = sign(&key, valid_claims());
        let mut parts: Vec<&str> = token.split('.').collect();
        let tampered_sig = "AAAA";
        parts[2] = tampered_sig;
        let bad = parts.join(".");

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        assert!(v.verify(&bad).await.is_err());
    }

    #[tokio::test]
    async fn rejects_unknown_kid() {
        let key = gen_key();
        let server = jwks_server(&key).await;
        let mut other = gen_key();
        other.kid = "different-kid".into();
        let token = sign(&other, valid_claims());

        let v = GoogleTokenVerifier::new("client-123".into(), server.uri());
        assert!(v.verify(&token).await.is_err());
    }
}
