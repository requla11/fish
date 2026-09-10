# Artifact Signing & Verification

Fish can sign every artifact it pushes to a remote cache and refuse any
download whose signature does not verify. This page explains how to set
that up end to end.

## Concepts

| Term | Meaning |
|---|---|
| **Seed** | 32 random bytes (64 hex chars). Derives the Ed25519 signing keypair. Keep it secret. |
| **Public key** | Derived from the seed via `fish signing-key`. Safe to share; used to verify signatures. |
| **Signature gate** | Middleware on the remote-cache client: signs uploads, verifies downloads. |

## 1. Generate a signing seed

```powershell
# Windows (PowerShell, .NET crypto RNG)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
```

```bash
# macOS / Linux
openssl rand -hex 32
```

Back the seed up in a password manager. Losing it means losing your
signing identity; leaking it means someone can forge your provenance.

## 2. Export the public key

```bash
export FISH_SIGNING_SEED=<your-64-hex-chars>
fish signing-key
# -> 87362bc246e5fe912fa774cfa728cece02545fc3ef7abae394c65e30a2da9455
```

The command prints only the public key — the seed never appears in
output.

## 3. Sign builds (producer side)

```bash
export FISH_SIGNING_SEED=<seed>
fish build            # uploads are now signed transparently
```

## 4. Verify artifacts (consumer side)

```bash
export FISH_SIGNING_SEED=<seed>                      # same identity, for re-signing pushes
export FISH_TRUSTED_KEYS=87362bc2...9455             # comma-separated list allowed
export FISH_SIG_POLICY=refuse                        # default; use "warn" to log instead of fail

fish build           # downloads failing verification are rejected
```

Policies:

- `refuse` (default): a download whose signature is missing/invalid or
  signed by an untrusted key fails the task.
- `warn`: accept but print a warning per offending artifact.

## CI integration

Set `RELEASE_SIGNING_SEED` as a repository secret; `.github/workflows/release.yaml`
signs SLSA provenance statements with it during releases. Consumers can
pin the release public key in their runners' `FISH_TRUSTED_KEYS`.

## Rotation

1. Generate a new seed.
2. Add the **new** public key to `FISH_TRUSTED_KEYS` alongside the old one.
3. Rebuild and republish artifacts.
4. Remove the old key from the trusted list once no consumer depends on it.
