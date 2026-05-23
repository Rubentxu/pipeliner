# Design: MVP-3 Credentials & Artifacts

## Technical Approach

Create a new `pipeliner-credentials` crate following existing crate patterns (pipeliner-library, pipeliner-steps-*). The approach uses a `CredentialProvider` trait similar to `StepFactory`, with concrete providers for file-based credentials and environment variables. The `withCredentials` step wraps execution with credential injection into EnvContext. JUnit support is added via enhanced `archive` step, and a `gc` CLI command manages artifact cleanup.

## Architecture Decisions

### Decision: CredentialProvider Trait Pattern

**Choice**: `trait CredentialProvider: Send + Sync` with `fn provide(&self, id: &str) -> Result<Credential, CredentialError>`
**Alternatives considered**: Direct enum variants (File, EnvVar, Vault), closure-based providers
**Rationale**: Follows existing StepFactory pattern (trait + Arc). Enables extensibility without modifying callers. Existing architecture score is 60 with cycles — must not worsen it.

### Decision: Secret Masking via EnvContext Extension

**Choice**: Add `masked_vars: HashSet<String>` to EnvContext + masking layer in interpolate/logging
**Alternatives considered**: Separate SecretContext, wrapper type for secret strings
**Rationale**: Minimal intrusion into existing EnvContext. Masking at interpolation + output prevents accidental secret exposure. Follows existing patterns.

### Decision: JUnit as Archive Enhancement

**Choice**: Add `format: JUnit` variant to archive step + `JUnitReport` struct
**Alternatives considered**: Separate junit step, new artifact tool
**Rationale**: Natural extension of existing archive semantics. Single step handles both artifact archival and test report processing.

## Data Flow

```
Pipeline Spec
    │
    ▼
┌──────────────────────────────────────────────────────────────┐
│ withCredentials Step                                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ For each credential binding:                            │ │
│  │   provider.provide(id) → Credential                     │ │
│  │   EnvContext.set(SCOPE_VAR, masked_value)               │ │
│  └────────────────────────────────────────────────────────┘ │
│  │ Inner Steps Execute (interpolation uses masked values)   │ │
│  │ EnvContext restored (masked values cleared)               │ │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
                       Artifact Storage
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ gc command: scan artifacts/ → delete if age > threshold      │
└──────────────────────────────────────────────────────────────┘
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/pipeliner-credentials/Cargo.toml` | Create | New crate with serde, once_cell deps |
| `crates/pipeliner-credentials/src/lib.rs` | Create | Public exports |
| `crates/pipeliner-credentials/src/provider.rs` | Create | CredentialProvider trait + error types |
| `crates/pipeliner-credentials/src/providers/file.rs` | Create | FileProvider implementation |
| `crates/pipeliner-credentials/src/providers/env.rs` | Create | EnvVarProvider implementation |
| `crates/pipeliner-credentials/src/masking.rs` | Create | SecretMasker utility |
| `crates/pipeliner-core/src/spec/step_spec.rs` | Modify | Add `WithCredentialsStepSpec` variant |
| `crates/pipeliner-core/src/spec/mod.rs` | Modify | Export WithCredentialsStepSpec |
| `crates/pipeliner-runtime/src/local_executor.rs` | Modify | Add execute_with_credentials_step_impl |
| `crates/pipeliner-steps-artifact/src/artifact_tool.rs` | Modify | Add JUnit format handling |
| `crates/pipeliner-cli/src/commands/gc.rs` | Create | New gc command module |
| `crates/pipeliner-cli/src/commands/mod.rs` | Modify | Register gc module |
| `crates/pipeliner-cli/src/main.rs` | Modify | Add Gc command variant |

## Entropy Constraints

| Interface | I(X;T) Leakage | I(T;Y) Coverage | Bottleneck Quality | SOLID Check |
|-----------|---------------|-----------------|-------------------|-------------|
| `CredentialProvider` | ~2.5 bits (file path exposure) | ~3 bits (id→value only) | Optimal | SRP OK ISP OK DIP OK |
| `SecretMasker` | ~1 bit (mask char count) | ~2 bits (hide/reveal) | Optimal | SRP OK ISP OK DIP OK |

**Interface Design Issues**: None
**SRP Split Candidates**: None
**ISP Violations**: None — CredentialProvider has single method
**DIP Assessment**: High-H abstraction (trait) depends on low-H (implementations), correct DIP
**Estimation Method**: Heuristic
**Confidence**: estimated

## Interfaces / Contracts

```rust
// CredentialProvider trait (crates/pipeliner-credentials/src/provider.rs)
pub trait CredentialProvider: Send + Sync {
    fn provide(&self, id: &str) -> Result<Credential, CredentialError>;
    fn scopes(&self) -> Vec<&str>;
}

#[derive(Debug, Clone)]
pub struct Credential {
    pub value: String,
    pub masked: bool,  // If true, value is pre-masked
}

#[derive(Debug, Clone)]
pub enum CredentialError {
    NotFound { id: String },
    ProviderError { message: String },
}

// WithCredentialsStepSpec (crates/pipeliner-core/src/spec/step_spec.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithCredentialsStepSpec {
    pub bindings: Vec<CredentialBinding>,
    pub steps: Vec<StepSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialBinding {
    pub variable: String,
    pub credentials_id: String,
    pub provider: Option<String>,  // If None, use default chain
}

// SecretMasker (crates/pipeliner-credentials/src/masking.rs)
pub struct SecretMasker {
    patterns: Vec<Regex>,
    masked_vars: HashSet<String>,
}

impl SecretMasker {
    pub fn mask(&self, input: &str) -> String { /* replace secrets with *** */ }
    pub fn register_variable(&mut self, var: String) { /* mark var as secret */ }
    pub fn is_masked(&self, var: &str) -> bool { /* check if var is masked */ }
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | CredentialProvider implementations | Mock providers, test file/env lookup |
| Unit | SecretMasker regex patterns | Test masking of known secret patterns |
| Unit | WithCredentialsStepSpec serde | JSON roundtrip |
| Integration | withCredentials step in pipeline | Execute pipeline with credentials |
| E2E | gc command artifact cleanup | Create artifacts, run gc, verify deletion |

## Migration / Rollout

No migration required. This is a pure addition (new crate, new step variant, new CLI command).

## Open Questions

- [ ] Should credentials be cached in memory during pipeline execution? Security vs performance tradeoff.
- [ ] What is the default credentials file location? `~/.pipeliner/credentials.toml`?
- [ ] Should gc command be integrated into archive step or remain standalone?
