# Lemon Squeezy API Reference for lamco-rdp-server

**Purpose:** Documentation for implementing optional license validation
**API Version:** v1
**Base URL:** https://api.lemonsqueezy.com/v1

---

## Overview

Lemon Squeezy provides a complete License API for:
- Validating license keys
- Activating license keys (with instance tracking)
- Deactivating license keys
- Managing activation limits

**Implementation Decision:** Honor system initially, soft validation optional later.

---

## API Endpoints

### 1. Validate License Key

**Endpoint:** `POST /v1/licenses/validate`

**Purpose:** Verify a license key is valid without consuming an activation slot

**Request:**
```bash
curl -X POST https://api.lemonsqueezy.com/v1/licenses/validate \
  -H "Accept: application/json" \
  -d "license_key=38b1460a-5104-4067-a91d-77b872934d51" \
  -d "instance_id=f90ec370-fd83-46a5-8bbd-44a241e78665"  # Optional
```

**Parameters:**
- `license_key` (required): The license key string to validate
- `instance_id` (optional): If provided, validates that specific instance is activated

**Response:**
```json
{
  "valid": true,
  "error": null,
  "license_key": {
    "id": 12345,
    "status": "active",
    "key": "38b1460a-5104-4067-a91d-77b872934d51",
    "activation_limit": 5,
    "activation_usage": 2,
    "created_at": "2025-01-01T00:00:00.000000Z",
    "expires_at": null
  },
  "instance": {
    "id": "f90ec370-fd83-46a5-8bbd-44a241e78665",
    "name": "Server-1",
    "created_at": "2025-01-15T10:30:00.000000Z"
  },
  "meta": {
    "store_id": 12345,
    "order_id": 67890,
    "product_id": 11111,
    "product_name": "lamco-rdp-server Annual License",
    "variant_id": 22222,
    "variant_name": "Annual Subscription",
    "customer_id": 33333,
    "customer_name": "John Doe",
    "customer_email": "john@example.com"
  }
}
```

**Error Response:**
```json
{
  "valid": false,
  "error": "License key not found",
  "license_key": null,
  "instance": null,
  "meta": null
}
```

**Common Errors:**
- `"License key not found"` - Invalid key
- `"License key expired"` - Key expired
- `"License key disabled"` - Key disabled by seller
- Instance not found (if instance_id provided)

---

### 2. Activate License Key

**Endpoint:** `POST /v1/licenses/activate`

**Purpose:** Activate a license key on a specific instance (server/machine)

**Request:**
```bash
curl -X POST https://api.lemonsqueezy.com/v1/licenses/activate \
  -H "Accept: application/json" \
  -d "license_key=38b1460a-5104-4067-a91d-77b872934d51" \
  -d "instance_name=production-server-1"
```

**Parameters:**
- `license_key` (required): The license key to activate
- `instance_name` (required): Human-readable name for this instance

**Response:**
```json
{
  "activated": true,
  "error": null,
  "license_key": { ... },
  "instance": {
    "id": "f90ec370-fd83-46a5-8bbd-44a241e78665",
    "name": "production-server-1",
    "created_at": "2025-01-15T10:30:00.000000Z"
  },
  "meta": { ... }
}
```

**Notes:**
- Creates a unique `instance_id` for this activation
- Increments `activation_usage` count
- Fails if `activation_limit` reached
- Returns existing instance if already activated with same name

---

### 3. Deactivate License Key

**Endpoint:** `POST /v1/licenses/deactivate`

**Purpose:** Deactivate a specific instance, freeing an activation slot

**Request:**
```bash
curl -X POST https://api.lemonsqueezy.com/v1/licenses/deactivate \
  -H "Accept: application/json" \
  -d "license_key=38b1460a-5104-4067-a91d-77b872934d51" \
  -d "instance_id=f90ec370-fd83-46a5-8bbd-44a241e78665"
```

**Parameters:**
- `license_key` (required): The license key
- `instance_id` (required): The instance ID to deactivate

**Response:**
```json
{
  "deactivated": true,
  "error": null,
  "license_key": { ... },
  "meta": { ... }
}
```

---

## Implementation Patterns

### Pattern 1: Honor System (Current Plan)

**No API calls - just LICENSE file**

```rust
// No code needed
// Just trust users to comply with LICENSE terms
```

**Pros:**
- Zero complexity
- No network calls
- No user friction
- Professional trust-based approach

**Cons:**
- No enforcement
- Relies on user honesty

---

### Pattern 2: Soft Validation (Optional CLI Flag)

**Validate if --license-key provided, warn if invalid**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ValidateRequest {
    license_key: String,
}

#[derive(Deserialize)]
struct ValidateResponse {
    valid: bool,
    error: Option<String>,
    meta: Option<LicenseMeta>,
}

#[derive(Deserialize)]
struct LicenseMeta {
    store_id: u64,
    product_id: u64,
}

async fn validate_license(key: &str) -> Result<bool> {
    let client = reqwest::Client::new();

    let resp = client
        .post("https://api.lemonsqueezy.com/v1/licenses/validate")
        .header("Accept", "application/json")
        .form(&ValidateRequest {
            license_key: key.to_string(),
        })
        .send()
        .await?;

    let data: ValidateResponse = resp.json().await?;

    if !data.valid {
        tracing::warn!("License validation failed: {:?}", data.error);
        return Ok(false);
    }

    // CRITICAL: Verify this is OUR product
    if let Some(meta) = data.meta {
        const EXPECTED_STORE_ID: u64 = YOUR_STORE_ID;
        const EXPECTED_PRODUCT_ID: u64 = YOUR_PRODUCT_ID;

        if meta.store_id != EXPECTED_STORE_ID || meta.product_id != EXPECTED_PRODUCT_ID {
            tracing::warn!("License key is for wrong product");
            return Ok(false);
        }
    }

    Ok(true)
}

// In main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(license_key) = args.license_key {
        match validate_license(&license_key).await {
            Ok(true) => {
                tracing::info!("✓ Commercial license validated");
            }
            Ok(false) => {
                tracing::warn!("⚠ License validation failed");
                tracing::warn!("Commercial use requires a valid license");
                tracing::warn!("Purchase at: https://yourstore.lemonsqueezy.com");
                // Continue running anyway (soft enforcement)
            }
            Err(e) => {
                tracing::warn!("Could not validate license: {}", e);
                // Continue running (don't fail on network errors)
            }
        }
    } else {
        // No license key provided - show info
        tracing::info!("Running without commercial license");
        tracing::info!("Free for non-profits and small businesses");
        tracing::info!("See LICENSE file or visit https://lamco.ai");
    }

    // Start server normally
    start_server(args).await
}
```

**Pros:**
- Gentle reminder for commercial users
- Validates genuine licenses
- Still works without license
- No user friction

**Cons:**
- Adds network dependency
- Slightly more complex

---

### Pattern 3: Activation-Based (Per-Server Licensing)

**Track activated servers, enforce activation limit**

```rust
async fn activate_license(key: &str, server_name: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let resp = client
        .post("https://api.lemonsqueezy.com/v1/licenses/activate")
        .header("Accept", "application/json")
        .form(&[
            ("license_key", key),
            ("instance_name", server_name),
        ])
        .send()
        .await?;

    let data: ActivateResponse = resp.json().await?;

    if !data.activated {
        bail!("License activation failed: {:?}", data.error);
    }

    // Store instance_id for future validation
    Ok(data.instance.id)
}

// Store instance_id in config file
// Validate on startup using Pattern 2 with instance_id
```

**Use Case:**
- Perpetual licenses: 1 activation = 1 server
- Annual licenses: Track which servers are using the subscription

**Pros:**
- Enforces "per server" licensing
- Prevents sharing of single license across many servers

**Cons:**
- More complex
- User friction (activation step)
- Need deactivation workflow

---

## Security Best Practices

### 1. Verify Product IDs ⚠️ CRITICAL

**Always check store_id and product_id:**

```rust
const EXPECTED_STORE_ID: u64 = 12345;      // Your Lemon Squeezy store ID
const EXPECTED_PRODUCT_ID: u64 = 67890;    // Your product ID

if meta.store_id != EXPECTED_STORE_ID || meta.product_id != EXPECTED_PRODUCT_ID {
    // Someone is using a license key from a different product!
    return Err("Invalid license");
}
```

**Why:** Prevents someone buying a cheap $5 product from same Lemon Squeezy store and using that license key.

### 2. Handle Network Failures Gracefully

```rust
match validate_license(&key).await {
    Err(NetworkError) => {
        // Don't block user if Lemon Squeezy is down
        tracing::warn!("Could not validate license (network error)");
        // Continue running
    }
    // ...
}
```

### 3. Cache Validation Results

```rust
// Don't validate on every startup
// Cache for 24 hours
let cache_path = dirs::cache_dir()
    .unwrap()
    .join("lamco-rdp-server/license-cache.json");

if let Ok(cached) = read_cache(&cache_path) {
    if cached.validated_at + Duration::days(1) > Utc::now() {
        // Use cached result
        return Ok(cached.valid);
    }
}

// Validate and update cache
```

### 4. No Hard Failures

**Never block the program** if validation fails (unless you want hard enforcement):

```rust
// GOOD: Soft enforcement
if !validate_license(&key).await? {
    println!("⚠ License validation failed. Commercial use requires a license.");
    println!("Purchase at: https://yourstore.lemonsqueezy.com");
    // Continue running
}

// BAD: Hard enforcement (user hostile)
if !validate_license(&key).await? {
    bail!("Invalid license. Exiting.");  // Don't do this!
}
```

---

## Rust Dependencies

For API integration, add to `Cargo.toml`:

```toml
[dependencies]
# For API calls
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# For caching (optional)
dirs = "5.0"
chrono = "0.4"
```

---

## Recommended Implementation Approach

### Phase 1: Honor System (Launch)
- No API calls
- Just LICENSE file terms
- Trust users

### Phase 2: Soft Validation (3-6 months later)
- Add `--license-key` optional flag
- Validate via API if provided
- Warn if invalid, but continue running
- Collect telemetry on adoption

### Phase 3: Activation Tracking (If needed)
- If seeing abuse, add activation system
- Enforce activation limits
- Provide deactivation workflow

**Start simple, add enforcement only if necessary.**

---

## API Rate Limits

**Not documented in Lemon Squeezy docs**

Conservative assumptions:
- Validate once per startup (not per request)
- Cache results for 24 hours
- Implement exponential backoff on failures

---

## Testing

### Test Mode

Lemon Squeezy provides test mode for development:

1. Create test products in dashboard
2. Generate test license keys
3. Test API without real payments

### Example Test

```rust
#[tokio::test]
async fn test_license_validation() {
    let test_key = "test-license-key-from-dashboard";
    let result = validate_license(test_key).await;
    assert!(result.is_ok());
}
```

---

## Documentation Links

Official Lemon Squeezy API documentation:

- **Validation Guide**: [Validating License Keys With the License API](https://docs.lemonsqueezy.com/guides/tutorials/license-keys)
- **License API Reference**: [License API Documentation](https://docs.lemonsqueezy.com/api/license-api)
- **Validate Endpoint**: [Validate a License Key](https://docs.lemonsqueezy.com/api/license-api/validate-license-key)
- **Activate Endpoint**: [Activate a License Key](https://docs.lemonsqueezy.com/api/license-api/activate-license-key)
- **Getting Started**: [Getting Started with the API](https://docs.lemonsqueezy.com/guides/developer-guide/getting-started)

---

## Decision for lamco-rdp-server

**Current Plan:** **Honor System (Pattern 1)**

**Rationale:**
- Focus on product quality first
- Build trust with users
- Avoid complexity and user friction
- Can add validation later if needed

**Implementation:** None - just LICENSE file

**Future:** Add Pattern 2 (soft validation) in v0.2 or v0.3 if needed

---

**Status:** Documentation complete, implementation deferred
**Last Updated:** 2025-12-21
