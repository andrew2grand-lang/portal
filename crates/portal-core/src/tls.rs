//! TLS 1.3 with mutual certificates and pinned fingerprints (brief §8).
//!
//! Every handshake verifies the peer's CertificateVerify signature against the
//! certificate it presented (`verify_tls13_signature`). In pinned mode the
//! certificate's SHA-256 must also equal the pinned fingerprint, compared in
//! constant time. Checking the fingerprint alone would let anyone who has
//! seen a peer's public certificate impersonate it.
//!
//! The server side always runs in open mode (any certificate whose signature
//! verifies), because it must accept first contact for pairing; the node then
//! decides what an unknown or known fingerprint may do.

use std::sync::{Arc, OnceLock};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, Error, SignatureScheme};

use crate::consts::ALPN;
use crate::identity::Identity;
use crate::model::Fingerprint;

/// SNI sent by clients. Names aren't checked: identity is the pinned key.
pub const SERVER_NAME: &str = "portal";

/// ring, TLS 1.3 suites only (rustls is also built without its `tls12` feature).
pub fn provider() -> Arc<CryptoProvider> {
    static PROVIDER: OnceLock<Arc<CryptoProvider>> = OnceLock::new();
    PROVIDER
        .get_or_init(|| {
            let mut p = rustls::crypto::ring::default_provider();
            p.cipher_suites.retain(|s| s.version() == &rustls::version::TLS13);
            Arc::new(p)
        })
        .clone()
}

fn chain_and_key(identity: &Identity) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    (
        vec![CertificateDer::from(identity.cert_der.clone())],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_der().to_vec())),
    )
}

pub fn server_config(identity: &Identity) -> crate::Result<Arc<rustls::ServerConfig>> {
    let (chain, key) = chain_and_key(identity);
    let mut config = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(Arc::new(PeerVerifier::new(None)))
        .with_single_cert(chain, key)?;
    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(Arc::new(config))
}

/// `pin = Some(fp)` for a trusted peer; `None` for first contact (pairing).
pub fn client_config(identity: &Identity, pin: Option<Fingerprint>) -> crate::Result<Arc<rustls::ClientConfig>> {
    let (chain, key) = chain_and_key(identity);
    let mut config = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PeerVerifier::new(pin)))
        .with_client_auth_cert(chain, key)?;
    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(Arc::new(config))
}

/// Fingerprint of the certificate the peer actually presented in this session.
pub fn peer_fingerprint(conn: &rustls::CommonState) -> Option<Fingerprint> {
    conn.peer_certificates().and_then(|c| c.first()).map(|c| Fingerprint::of_cert(c))
}

#[derive(Debug)]
struct PeerVerifier {
    pin: Option<Fingerprint>,
    algorithms: WebPkiSupportedAlgorithms,
}

impl PeerVerifier {
    fn new(pin: Option<Fingerprint>) -> Self {
        Self { pin, algorithms: provider().signature_verification_algorithms }
    }

    fn check(&self, end_entity: &CertificateDer<'_>) -> Result<(), Error> {
        match &self.pin {
            Some(pin) if !Fingerprint::of_cert(end_entity).ct_eq(pin) => {
                Err(Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure))
            }
            _ => Ok(()),
        }
    }

    fn verify13(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }
}

impl ServerCertVerifier for PeerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        self.check(end_entity).map(|_| ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.verify13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

impl ClientCertVerifier for PeerVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        self.check(end_entity).map(|_| ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.verify13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }
}
