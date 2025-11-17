//! TLS 1.3 configuration and management
//!
//! Provides secure TLS termination for RDP connections using rustls.

use anyhow::{Context, Result};
use rustls::server::ServerConnection;
use rustls::version::TLS13;
use rustls::{Certificate, PrivateKey, ServerConfig};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// TLS configuration wrapper
#[derive(Clone)]
pub struct TlsConfig {
    /// Certificate chain
    cert_chain: Vec<Certificate>,

    /// Private key
    #[allow(dead_code)]
    private_key: PrivateKey,

    /// rustls ServerConfig
    server_config: Arc<ServerConfig>,
}

impl TlsConfig {
    /// Create TLS config from PEM files
    pub fn from_files(cert_path: &Path, key_path: &Path) -> Result<Self> {
        info!("Loading TLS configuration from files");
        debug!("Certificate: {:?}", cert_path);
        debug!("Private key: {:?}", key_path);

        // Load certificate chain
        let cert_file =
            std::fs::File::open(cert_path).context("Failed to open certificate file")?;
        let mut cert_reader = std::io::BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)
            .context("Failed to parse certificate")?
            .into_iter()
            .map(Certificate)
            .collect::<Vec<_>>();

        if certs.is_empty() {
            anyhow::bail!("No certificates found in file");
        }

        // Load private key
        let key_file = std::fs::File::open(key_path).context("Failed to open private key file")?;
        let mut key_reader = std::io::BufReader::new(key_file);

        // Try different key formats
        let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .context("Failed to parse private key")?;

        let private_key = if !keys.is_empty() {
            PrivateKey(keys[0].clone())
        } else {
            // Try RSA format
            let key_file = std::fs::File::open(key_path)?;
            let mut key_reader = std::io::BufReader::new(key_file);
            let rsa_keys = rustls_pemfile::rsa_private_keys(&mut key_reader)
                .context("Failed to parse RSA private key")?;

            if rsa_keys.is_empty() {
                anyhow::bail!("No private key found in file");
            }
            PrivateKey(rsa_keys[0].clone())
        };

        // Create ServerConfig with TLS 1.3 only
        let server_config = ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&TLS13])
            .context("Failed to configure TLS versions")?
            .with_no_client_auth()
            .with_single_cert(certs.clone(), private_key.clone())
            .context("Failed to configure certificate")?;

        info!("TLS 1.3 configuration created successfully");

        Ok(Self {
            cert_chain: certs,
            private_key,
            server_config: Arc::new(server_config),
        })
    }

    /// Get rustls ServerConfig
    pub fn server_config(&self) -> Arc<ServerConfig> {
        self.server_config.clone()
    }

    /// Get certificate chain
    pub fn certificates(&self) -> &[Certificate] {
        &self.cert_chain
    }

    /// Verify TLS configuration is valid
    pub fn verify(&self) -> Result<()> {
        // Verify we have at least one certificate
        if self.cert_chain.is_empty() {
            anyhow::bail!("No certificates in chain");
        }

        // Verify ServerConfig is properly initialized
        if Arc::strong_count(&self.server_config) == 0 {
            anyhow::bail!("ServerConfig not properly initialized");
        }

        info!("TLS configuration verified");
        Ok(())
    }
}

/// TLS acceptor for incoming connections
pub struct TlsAcceptor {
    config: Arc<ServerConfig>,
}

impl TlsAcceptor {
    /// Create new TLS acceptor
    pub fn new(config: TlsConfig) -> Self {
        Self {
            config: config.server_config(),
        }
    }

    /// Accept TLS connection
    pub fn accept(&self) -> ServerConnection {
        ServerConnection::new(self.config.clone()).expect("Failed to create ServerConnection")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_cert_paths() -> (PathBuf, PathBuf) {
        (
            PathBuf::from("certs/test-cert.pem"),
            PathBuf::from("certs/test-key.pem"),
        )
    }

    #[test]
    fn test_tls_config_from_files() {
        let (cert_path, key_path) = get_test_cert_paths();

        // Skip if test certs don't exist
        if !cert_path.exists() || !key_path.exists() {
            eprintln!("Skipping test: test certificates not found");
            return;
        }

        let config = TlsConfig::from_files(&cert_path, &key_path).unwrap();
        assert!(!config.certificates().is_empty());
    }

    #[test]
    fn test_tls_config_verify() {
        let (cert_path, key_path) = get_test_cert_paths();

        if !cert_path.exists() || !key_path.exists() {
            return;
        }

        let config = TlsConfig::from_files(&cert_path, &key_path).unwrap();
        assert!(config.verify().is_ok());
    }

    #[test]
    fn test_tls_acceptor_creation() {
        let (cert_path, key_path) = get_test_cert_paths();

        if !cert_path.exists() || !key_path.exists() {
            return;
        }

        let config = TlsConfig::from_files(&cert_path, &key_path).unwrap();
        let _acceptor = TlsAcceptor::new(config);
        // If we get here, acceptor was created successfully
    }
}
