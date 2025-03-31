#[macro_use]
extern crate log;

use sdre_rust_logging::SetupLogging;
use std::env::args;
use std::error::Error;
use std::net::ToSocketAddrs;
use tunnel::Client;
use tunnel::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tunnel::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tunnel::rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};

#[derive(Debug)]
struct Verifier {}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<
        tunnel::rustls::client::danger::ServerCertVerified,
        tunnel::rustls::Error,
    > {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<
        tunnel::rustls::client::danger::HandshakeSignatureValid,
        tunnel::rustls::Error,
    > {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<
        tunnel::rustls::client::danger::HandshakeSignatureValid,
        tunnel::rustls::Error,
    > {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    "INFO".enable_logging();

    let args = args().collect::<Vec<_>>();
    if args.len() < 3 {
        error!("Usage: {} [bind_to] [upstream] [hostname]", &args[0]);
        return Ok(());
    }

    let client_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(Verifier {}))
        .with_no_client_auth();

    let hostname = args[3].to_string();

    let bind_to = match args[1].to_socket_addrs() {
        Ok(mut addr) => match addr.next() {
            Some(addr) => addr,
            None => {
                error!("No valid bind address found.");
                return Err("No valid bind address found.".into());
            }
        },
        Err(e) => {
            error!("Invalid bind address: {}", &args[1]);
            return Err(e.into());
        }
    };

    let upstream = match args[2].to_socket_addrs() {
        Ok(mut addr) => match addr.next() {
            Some(addr) => addr,
            None => {
                error!("No valid upstream address found.");
                return Err("No valid upstream address found.".into());
            }
        },
        Err(e) => {
            error!("Invalid upstream address: {}", &args[2]);
            return Err(e.into());
        }
    };

    let mut client = match Client::start(hostname.clone(), bind_to, upstream, client_config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to start client: {}", e);
            return Err(e);
        }
    };

    info!(
        "Listening on {} with upstream {} ({}).",
        bind_to, hostname, upstream,
    );

    loop {
        let (meta, pipes) = match client.wait_for_session().await {
            Ok(session) => session,
            Err(e) => {
                error!("Failed to create session: {}", e);
                continue;
            }
        };

        let (mut inbound, mut outbound) = pipes;

        tokio::spawn(async move { inbound.run().await });
        tokio::spawn(async move { outbound.run().await });

        info!(
            "Connection established: {} (TLS) <-> {}.",
            upstream,
            meta.get_peer_addr(),
        );
    }
}
