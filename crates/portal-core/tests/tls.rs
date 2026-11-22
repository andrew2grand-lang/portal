//! TLS 1.3 with pinned certificates (brief §8).

use std::sync::Arc;

use portal_core::identity::Identity;
use portal_core::model::Fingerprint;
use portal_core::tls;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{TlsAcceptor, TlsConnector};

fn identity() -> (tempfile::TempDir, Identity) {
    let dir = tempfile::tempdir().unwrap();
    let id = Identity::load_or_create(dir.path()).unwrap();
    (dir, id)
}

/// Runs one handshake over an in-memory pipe. Returns the fingerprint each
/// side saw for the other, or the client/server error text.
async fn handshake(
    server: Arc<rustls::ServerConfig>,
    client: Arc<rustls::ClientConfig>,
) -> Result<(Fingerprint, Fingerprint), String> {
    let (a, b) = tokio::io::duplex(64 * 1024);
    let server_task = tokio::spawn(async move {
        let mut s = TlsAcceptor::from(server).accept(a).await.map_err(|e| format!("server: {e}"))?;
        let mut byte = [0u8; 1];
        s.read_exact(&mut byte).await.map_err(|e| format!("server read: {e}"))?;
        s.write_all(&byte).await.map_err(|e| format!("server write: {e}"))?;
        s.flush().await.map_err(|e| format!("server flush: {e}"))?;
        let fp = tls::peer_fingerprint(s.get_ref().1).ok_or("server saw no client cert")?;
        Ok::<_, String>((fp, s))
    });
    let name = ServerName::try_from(tls::SERVER_NAME).unwrap();
    let client_result = async {
        let mut c = TlsConnector::from(client).connect(name, b).await.map_err(|e| format!("client: {e}"))?;
        c.write_all(&[42]).await.map_err(|e| format!("client write: {e}"))?;
        c.flush().await.map_err(|e| format!("client flush: {e}"))?;
        let mut byte = [0u8; 1];
        c.read_exact(&mut byte).await.map_err(|e| format!("client read: {e}"))?;
        assert_eq!(byte[0], 42);
        let fp = tls::peer_fingerprint(c.get_ref().1).ok_or("client saw no server cert")?;
        Ok::<_, String>((fp, c))
    }
    .await;
    match client_result {
        Ok((client_saw, _c)) => {
            let (server_saw, _s) = server_task.await.unwrap()?;
            Ok((client_saw, server_saw))
        }
        Err(e) => {
            server_task.abort();
            Err(e)
        }
    }
}

/// A server that presents `cert_of`'s certificate but signs with `key_of`'s key.
#[derive(Debug)]
struct Forged(Arc<CertifiedKey>);
impl ResolvesServerCert for Forged {
    fn resolve(&self, _: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}

fn certified(cert_of: &Identity, key_of: &Identity) -> Arc<CertifiedKey> {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_of.key_der().to_vec()));
    let signer = tls::provider().key_provider.load_private_key(key).unwrap();
    Arc::new(CertifiedKey::new(vec![CertificateDer::from(cert_of.cert_der.clone())], signer))
}

#[tokio::test]
async fn pinned_client_connects_to_the_right_server_and_both_see_the_real_fingerprints() {
    let ((_d1, server), (_d2, client)) = (identity(), identity());
    let result =
        handshake(tls::server_config(&server).unwrap(), tls::client_config(&client, Some(server.fingerprint)).unwrap())
            .await
            .unwrap();
    assert_eq!(result, (server.fingerprint, client.fingerprint));
}

#[tokio::test]
async fn pinned_client_rejects_a_different_server() {
    let ((_d1, server), (_d2, client), (_d3, other)) = (identity(), identity(), identity());
    let err =
        handshake(tls::server_config(&server).unwrap(), tls::client_config(&client, Some(other.fingerprint)).unwrap())
            .await
            .unwrap_err();
    assert!(err.starts_with("client"), "{err}");
}

#[tokio::test]
async fn forged_server_certificate_with_the_right_fingerprint_but_wrong_key_is_rejected() {
    let ((_d1, victim), (_d2, attacker), (_d3, client)) = (identity(), identity(), identity());
    // The attacker replays the victim's public certificate (so the fingerprint
    // matches the pin) but can only sign with its own key.
    let mut forged = (*tls::server_config(&attacker).unwrap()).clone();
    forged.cert_resolver = Arc::new(Forged(certified(&victim, &attacker)));

    let err =
        handshake(Arc::new(forged), tls::client_config(&client, Some(victim.fingerprint)).unwrap()).await.unwrap_err();

    assert!(err.starts_with("client"), "{err}");
    assert!(err.to_lowercase().contains("signature") || err.to_lowercase().contains("decrypt"), "{err}");
}

#[tokio::test]
async fn forged_client_certificate_with_the_right_fingerprint_but_wrong_key_is_rejected() {
    let ((_d1, server), (_d2, victim), (_d3, attacker)) = (identity(), identity(), identity());
    // The attacker presents the victim's certificate but signs CertificateVerify with its own key.
    let mut forged = (*tls::client_config(&attacker, Some(server.fingerprint)).unwrap()).clone();
    forged.client_auth_cert_resolver = Arc::new(StaticClient(certified(&victim, &attacker)));

    let err = handshake(tls::server_config(&server).unwrap(), Arc::new(forged)).await.unwrap_err();

    // The server aborts the handshake; the client may notice first or second.
    assert!(err.starts_with("server") || err.starts_with("client"), "{err}");
}

#[derive(Debug)]
struct StaticClient(Arc<CertifiedKey>);
impl rustls::client::ResolvesClientCert for StaticClient {
    fn resolve(&self, _: &[&[u8]], _: &[rustls::SignatureScheme]) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
    fn has_certs(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn open_client_accepts_any_genuine_server_for_pairing() {
    let ((_d1, server), (_d2, client)) = (identity(), identity());
    let (client_saw, _) =
        handshake(tls::server_config(&server).unwrap(), tls::client_config(&client, None).unwrap()).await.unwrap();
    assert_eq!(client_saw, server.fingerprint);
}

#[tokio::test]
async fn open_client_still_rejects_a_server_that_cannot_sign_for_its_certificate() {
    let ((_d1, victim), (_d2, attacker), (_d3, client)) = (identity(), identity(), identity());
    let mut forged = (*tls::server_config(&attacker).unwrap()).clone();
    forged.cert_resolver = Arc::new(Forged(certified(&victim, &attacker)));
    assert!(handshake(Arc::new(forged), tls::client_config(&client, None).unwrap()).await.is_err());
}

#[test]
fn only_tls13_and_the_portal_alpn_are_configured() {
    let (_d, id) = identity();
    let client = tls::client_config(&id, None).unwrap();
    let server = tls::server_config(&id).unwrap();
    assert_eq!(client.alpn_protocols, vec![b"portal/1".to_vec()]);
    assert_eq!(server.alpn_protocols, vec![b"portal/1".to_vec()]);
    assert!(tls::provider().cipher_suites.iter().all(|s| s.version() == &rustls::version::TLS13));
}
