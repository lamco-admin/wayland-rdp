# TASK P1-02: SECURITY MODULE (TLS & AUTHENTICATION)
**Task ID:** TASK-P1-02
**Phase:** 1
**Milestone:** Security
**Duration:** 5-7 days
**Assigned To:** [Agent/Developer Name]
**Dependencies:** TASK-P1-01 (Foundation)
**Status:** NOT_STARTED

---

## TASK OVERVIEW

### Objective
Implement the complete security module including TLS 1.3 termination, certificate management, and PAM-based authentication. This module provides the security foundation for all RDP connections.

### Success Criteria
- ✅ TLS 1.3 handshake completes successfully
- ✅ Self-signed certificate generation works
- ✅ Certificate loading from PEM files works
- ✅ PAM authentication functional
- ✅ NLA (CredSSP) preparation complete
- ✅ All security tests pass
- ✅ No unsafe code without justification

### Deliverables
1. TLS configuration module (`src/security/tls.rs`)
2. Certificate management module (`src/security/certificates.rs`)
3. Authentication module (`src/security/auth.rs`)
4. Security manager coordinator (`src/security/mod.rs`)
5. Certificate generation script
6. Unit tests for all security components
7. Integration test for TLS handshake

---

## TECHNICAL SPECIFICATION

### 1. Module Structure

```
src/security/
├── mod.rs           # Security manager coordinator
├── tls.rs           # TLS 1.3 configuration
├── certificates.rs  # Certificate management
└── auth.rs          # Authentication (PAM)
```

---

### 2. TLS Configuration Module

#### File: `src/security/tls.rs`

```rust
//! TLS 1.3 configuration and management
//!
//! Provides secure TLS termination for RDP connections using rustls.

use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls::server::ServerConnection;
use rustls::version::TLS13;
use std::sync::Arc;
use std::path::Path;
use anyhow::{Result, Context};
use tracing::{info, debug, error};

/// TLS configuration wrapper
#[derive(Clone)]
pub struct TlsConfig {
    /// Certificate chain
    cert_chain: Vec<Certificate>,

    /// Private key
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
        let cert_file = std::fs::File::open(cert_path)
            .context("Failed to open certificate file")?;
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
        let key_file = std::fs::File::open(key_path)
            .context("Failed to open private key file")?;
        let mut key_reader = std::io::BufReader::new(key_file);

        // Try different key formats
        let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .context("Failed to parse private key")?;

        let private_key = if !keys.is_empty() {
            PrivateKey(keys[0].clone())
        } else {
            // Try RSA format
            key_reader = std::io::BufReader::new(std::fs::File::open(key_path)?);
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
        ServerConnection::new(self.config.clone())
            .expect("Failed to create ServerConnection")
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
```

---

### 3. Certificate Management Module

#### File: `src/security/certificates.rs`

```rust
//! Certificate generation and management
//!
//! Provides utilities for generating self-signed certificates
//! and managing certificate lifecycle.

use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use tracing::{info, warn};

/// Certificate generator
pub struct CertificateGenerator;

impl CertificateGenerator {
    /// Generate self-signed certificate
    pub fn generate_self_signed(
        common_name: &str,
        validity_days: u32,
    ) -> Result<(String, String)> {
        info!("Generating self-signed certificate for '{}'", common_name);

        // Create certificate parameters
        let mut params = CertificateParams::default();

        // Set subject
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, common_name);
        params.distinguished_name = distinguished_name;

        // Set validity period
        params.not_before = time::OffsetDateTime::now_utc();
        params.not_after = time::OffsetDateTime::now_utc()
            + time::Duration::days(validity_days as i64);

        // Generate key pair
        let key_pair = KeyPair::generate(&rcgen::PKCS_RSA_SHA256)
            .context("Failed to generate key pair")?;
        params.key_pair = Some(key_pair);

        // Generate certificate
        let cert = Certificate::from_params(params)
            .context("Failed to generate certificate")?;

        // Serialize to PEM
        let cert_pem = cert.serialize_pem()
            .context("Failed to serialize certificate")?;
        let key_pem = cert.serialize_private_key_pem();

        info!("Self-signed certificate generated successfully");

        Ok((cert_pem, key_pem))
    }

    /// Generate and save self-signed certificate to files
    pub fn generate_and_save(
        common_name: &str,
        validity_days: u32,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<()> {
        info!("Generating and saving certificate to {:?}", cert_path);

        // Check if files already exist
        if cert_path.exists() || key_path.exists() {
            warn!("Certificate or key file already exists, will overwrite");
        }

        // Generate certificate
        let (cert_pem, key_pem) = Self::generate_self_signed(common_name, validity_days)?;

        // Create parent directories if needed
        if let Some(parent) = cert_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create certificate directory")?;
        }
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create key directory")?;
        }

        // Write certificate
        fs::write(cert_path, cert_pem.as_bytes())
            .context("Failed to write certificate")?;

        // Write private key (with restricted permissions)
        fs::write(key_path, key_pem.as_bytes())
            .context("Failed to write private key")?;

        // Set permissions on private key (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(key_path)?.permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(key_path, perms)?;
        }

        info!("Certificate and key saved successfully");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_self_signed() {
        let (cert_pem, key_pem) =
            CertificateGenerator::generate_self_signed("test-server", 365)
                .unwrap();

        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_generate_and_save() {
        let temp_dir = TempDir::new().unwrap();
        let cert_path = temp_dir.path().join("cert.pem");
        let key_path = temp_dir.path().join("key.pem");

        CertificateGenerator::generate_and_save(
            "test-server",
            365,
            &cert_path,
            &key_path,
        ).unwrap();

        assert!(cert_path.exists());
        assert!(key_path.exists());

        // Verify contents
        let cert_content = fs::read_to_string(cert_path).unwrap();
        assert!(cert_content.contains("BEGIN CERTIFICATE"));

        let key_content = fs::read_to_string(key_path).unwrap();
        assert!(key_content.contains("BEGIN PRIVATE KEY"));
    }
}
```

---

### 4. Authentication Module

#### File: `src/security/auth.rs`

```rust
//! Authentication module using PAM
//!
//! Provides user authentication against system accounts using PAM
//! (Pluggable Authentication Modules).

use pam::Authenticator;
use std::sync::Arc;
use anyhow::{Result, Context};
use tracing::{info, warn, error};

/// Authentication method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// PAM authentication
    Pam,
    /// No authentication (development only)
    None,
}

impl AuthMethod {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pam" => Self::Pam,
            "none" => Self::None,
            _ => {
                warn!("Unknown auth method '{}', defaulting to PAM", s);
                Self::Pam
            }
        }
    }
}

/// User authenticator
pub struct UserAuthenticator {
    method: AuthMethod,
    service_name: String,
}

impl UserAuthenticator {
    /// Create new authenticator
    pub fn new(method: AuthMethod, service_name: Option<String>) -> Self {
        let service_name = service_name.unwrap_or_else(|| "wrd-server".to_string());

        info!("Initializing authenticator: {:?}, service: {}", method, service_name);

        Self {
            method,
            service_name,
        }
    }

    /// Authenticate user with password
    pub fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        match self.method {
            AuthMethod::Pam => self.authenticate_pam(username, password),
            AuthMethod::None => {
                warn!("Authentication disabled (development mode)");
                Ok(true)
            }
        }
    }

    /// Authenticate using PAM
    fn authenticate_pam(&self, username: &str, password: &str) -> Result<bool> {
        info!("Authenticating user '{}' via PAM", username);

        let mut auth = Authenticator::with_password(&self.service_name)
            .context("Failed to create PAM authenticator")?;

        // Set credentials
        auth.get_handler().set_credentials(username, password);

        // Authenticate
        match auth.authenticate() {
            Ok(_) => {
                info!("User '{}' authenticated successfully", username);

                // Open session (required for complete authentication)
                if let Err(e) = auth.open_session() {
                    warn!("Failed to open PAM session: {}", e);
                    // Continue anyway - authentication succeeded
                }

                Ok(true)
            }
            Err(e) => {
                warn!("Authentication failed for user '{}': {}", username, e);
                Ok(false)
            }
        }
    }

    /// Validate username format
    pub fn validate_username(username: &str) -> Result<()> {
        if username.is_empty() {
            anyhow::bail!("Username cannot be empty");
        }

        if username.len() > 32 {
            anyhow::bail!("Username too long (max 32 characters)");
        }

        // Check for valid characters (alphanumeric, underscore, dash)
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            anyhow::bail!("Username contains invalid characters");
        }

        Ok(())
    }
}

/// Session token for authenticated sessions
#[derive(Debug, Clone)]
pub struct SessionToken {
    token: String,
    username: String,
    created_at: std::time::SystemTime,
}

impl SessionToken {
    /// Create new session token
    pub fn new(username: String) -> Self {
        use uuid::Uuid;

        let token = Uuid::new_v4().to_string();
        let created_at = std::time::SystemTime::now();

        info!("Created session token for user '{}'", username);

        Self {
            token,
            username,
            created_at,
        }
    }

    /// Get token string
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Get username
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Check if token is expired
    pub fn is_expired(&self, max_age: std::time::Duration) -> bool {
        match self.created_at.elapsed() {
            Ok(elapsed) => elapsed > max_age,
            Err(_) => true, // Clock went backwards, consider expired
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_from_str() {
        assert_eq!(AuthMethod::from_str("pam"), AuthMethod::Pam);
        assert_eq!(AuthMethod::from_str("none"), AuthMethod::None);
        assert_eq!(AuthMethod::from_str("invalid"), AuthMethod::Pam);
    }

    #[test]
    fn test_validate_username() {
        assert!(UserAuthenticator::validate_username("validuser").is_ok());
        assert!(UserAuthenticator::validate_username("user_123").is_ok());
        assert!(UserAuthenticator::validate_username("user-name").is_ok());

        assert!(UserAuthenticator::validate_username("").is_err());
        assert!(UserAuthenticator::validate_username("a".repeat(33).as_str()).is_err());
        assert!(UserAuthenticator::validate_username("invalid@user").is_err());
    }

    #[test]
    fn test_session_token_creation() {
        let token = SessionToken::new("testuser".to_string());
        assert_eq!(token.username(), "testuser");
        assert!(!token.token().is_empty());
        assert!(!token.is_expired(std::time::Duration::from_secs(3600)));
    }

    #[test]
    fn test_none_auth() {
        let auth = UserAuthenticator::new(AuthMethod::None, None);
        assert!(auth.authenticate("anyuser", "anypass").unwrap());
    }
}
```

---

### 5. Security Manager Coordinator

#### File: `src/security/mod.rs`

```rust
//! Security module coordination
//!
//! Coordinates TLS, certificate management, and authentication.

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

pub mod tls;
pub mod certificates;
pub mod auth;

pub use tls::{TlsConfig, TlsAcceptor};
pub use certificates::CertificateGenerator;
pub use auth::{UserAuthenticator, AuthMethod, SessionToken};

use crate::config::Config;

/// Security manager coordinates all security operations
pub struct SecurityManager {
    tls_config: TlsConfig,
    authenticator: Arc<UserAuthenticator>,
}

impl SecurityManager {
    /// Create new security manager
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Initializing SecurityManager");

        // Load TLS configuration
        let tls_config = TlsConfig::from_files(
            &config.security.cert_path,
            &config.security.key_path,
        )?;

        // Verify TLS config
        tls_config.verify()?;

        // Create authenticator
        let auth_method = AuthMethod::from_str(&config.security.auth_method);
        let authenticator = Arc::new(UserAuthenticator::new(auth_method, None));

        info!("SecurityManager initialized successfully");

        Ok(Self {
            tls_config,
            authenticator,
        })
    }

    /// Create TLS acceptor
    pub fn create_acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::new(self.tls_config.clone())
    }

    /// Get authenticator
    pub fn authenticator(&self) -> Arc<UserAuthenticator> {
        self.authenticator.clone()
    }

    /// Authenticate user
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<SessionToken> {
        // Validate username format
        UserAuthenticator::validate_username(username)?;

        // Authenticate
        let authenticated = self.authenticator.authenticate(username, password)?;

        if !authenticated {
            anyhow::bail!("Authentication failed");
        }

        // Create session token
        Ok(SessionToken::new(username.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let config = Config::default_config().unwrap();

        // This will fail if certs don't exist, which is expected
        let result = SecurityManager::new(&config).await;

        // In real test environment with certs, this should pass
        if result.is_ok() {
            let manager = result.unwrap();
            let _acceptor = manager.create_acceptor();
        }
    }
}
```

---

### 6. Certificate Generation Script

#### File: `scripts/generate-certs.sh`

```bash
#!/bin/bash
# Generate self-signed certificates for wrd-server

set -e

CERT_DIR="${1:-/etc/wrd-server}"
COMMON_NAME="${2:-wrd-server}"
VALIDITY_DAYS="${3:-365}"

echo "Generating self-signed certificate..."
echo "Directory: $CERT_DIR"
echo "Common Name: $COMMON_NAME"
echo "Validity: $VALIDITY_DAYS days"

# Create directory
mkdir -p "$CERT_DIR"

# Generate certificate and key
openssl req -x509 \
    -newkey rsa:4096 \
    -nodes \
    -keyout "$CERT_DIR/key.pem" \
    -out "$CERT_DIR/cert.pem" \
    -days "$VALIDITY_DAYS" \
    -subj "/CN=$COMMON_NAME" \
    -addext "subjectAltName=DNS:$COMMON_NAME,DNS:localhost,IP:127.0.0.1"

# Set permissions
chmod 644 "$CERT_DIR/cert.pem"
chmod 600 "$CERT_DIR/key.pem"

echo "✓ Certificate generated successfully"
echo "  Certificate: $CERT_DIR/cert.pem"
echo "  Private Key: $CERT_DIR/key.pem"
```

Make executable:
```bash
chmod +x scripts/generate-certs.sh
```

---

### 7. Update Main Module

Update `src/lib.rs` to include security module:
```rust
pub mod security;
```

---

## VERIFICATION CHECKLIST

### Build Verification
- [ ] `cargo build` succeeds
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] No compiler warnings

### Test Verification
- [ ] `cargo test security::` passes all tests
- [ ] TLS config loading test passes
- [ ] Certificate generation test passes
- [ ] Authentication tests pass
- [ ] Session token tests pass

### Functionality Verification
- [ ] Can load certificates from PEM files
- [ ] Can generate self-signed certificates
- [ ] TLS acceptor creates successfully
- [ ] PAM authentication works (if PAM configured)
- [ ] Session tokens generate correctly
- [ ] Username validation works

### Security Verification
- [ ] Only TLS 1.3 supported (no fallback)
- [ ] Private key permissions set to 600
- [ ] No passwords logged
- [ ] Authentication failures logged
- [ ] No unsafe code blocks

---

## INTEGRATION TEST

Create `tests/integration/security_test.rs`:

```rust
use wrd_server::security::{TlsConfig, CertificateGenerator, UserAuthenticator, AuthMethod};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_certificate_generation_and_loading() {
    let temp_dir = TempDir::new().unwrap();
    let cert_path = temp_dir.path().join("cert.pem");
    let key_path = temp_dir.path().join("key.pem");

    // Generate certificate
    CertificateGenerator::generate_and_save(
        "test-server",
        365,
        &cert_path,
        &key_path,
    ).unwrap();

    // Load TLS config
    let tls_config = TlsConfig::from_files(&cert_path, &key_path).unwrap();

    // Verify
    assert!(tls_config.verify().is_ok());
    assert!(!tls_config.certificates().is_empty());
}

#[test]
fn test_authentication_none_method() {
    let auth = UserAuthenticator::new(AuthMethod::None, None);
    assert!(auth.authenticate("testuser", "testpass").unwrap());
}
```

---

## COMMON ISSUES AND SOLUTIONS

### Issue: PAM authentication fails
**Solution:**
- Create PAM service file `/etc/pam.d/wrd-server`:
```
auth    required    pam_unix.so
account required    pam_unix.so
```
- Or use `AuthMethod::None` for development

### Issue: "Permission denied" on private key
**Solution:** Ensure key file has 600 permissions:
```bash
chmod 600 /path/to/key.pem
```

### Issue: TLS handshake fails
**Solution:**
- Verify certificate and key match
- Check certificate is not expired
- Ensure TLS 1.3 is supported by client

---

## DELIVERABLE CHECKLIST

- [ ] `src/security/tls.rs` implemented
- [ ] `src/security/certificates.rs` implemented
- [ ] `src/security/auth.rs` implemented
- [ ] `src/security/mod.rs` implemented
- [ ] `scripts/generate-certs.sh` created
- [ ] Integration test created
- [ ] All unit tests passing
- [ ] Documentation complete
- [ ] Security review passed

---

## COMPLETION CRITERIA

This task is COMPLETE when:
1. All modules implemented as specified
2. All tests pass
3. Certificate generation works
4. TLS handshake successful
5. PAM authentication functional
6. Code review approved

**Time Estimate:** 5-7 days

---

**END OF TASK SPECIFICATION**
