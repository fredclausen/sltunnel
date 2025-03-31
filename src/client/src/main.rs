#[macro_use]
extern crate log;

use rustls_platform_verifier::ConfigVerifierExt;
use sdre_rust_logging::SetupLogging;
use std::env::args;
use std::error::Error;
use std::net::ToSocketAddrs;
use tunnel::Client;
use tunnel::rustls::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    "INFO".enable_logging();

    let args = args().collect::<Vec<_>>();
    if args.len() < 3 {
        error!("Usage: {} [bind_to] [upstream] [hostname]", &args[0]);
        return Ok(());
    }

    let client_config = ClientConfig::with_platform_verifier();

    info!("Loaded certificates.");

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
        let (meta, pipes) = client.wait_for_session().await?;
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
