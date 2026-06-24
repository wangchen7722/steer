# Spec — `marketplace-resolution`

> Capability introduced by change `workflow-marketplace-install`.
> Covers how `steer workflow install` determines which git repository to fetch:
> the `--marketplace` flag, the named registry (`.steer/marketplaces.toml`),
> and the `STEER_MARKETPLACE_URL` environment variable, including precedence and
> the error path when no source is configured.

## ADDED Requirements

### Requirement: A direct URL in `--marketplace` is used as-is and wins

When `--marketplace <value>` is passed and `<value>` looks like a URL — it
contains `://` or ends with `.git` — the installer MUST treat `<value>` as the
marketplace repository URL and clone it directly, ignoring any named-registry
entry and the `STEER_MARKETPLACE_URL` environment variable. A direct URL has the
highest precedence.

#### Scenario: an https URL is cloned directly

- **WHEN** the user runs `steer workflow install --marketplace https://github.com/foo/bar`
- **THEN** the installer clones `https://github.com/foo/bar` and does not read
  the registry file or the `STEER_MARKETPLACE_URL` environment variable.

#### Scenario: a URL ending in `.git` is cloned directly

- **WHEN** the user runs `steer workflow install --marketplace git@github.com:foo/bar.git`
- **THEN** the installer clones `git@github.com:foo/bar.git` directly.

---

### Requirement: A named `--marketplace` is resolved against the registry

When `--marketplace <name>` is passed and `<name>` does not look like a URL, the
installer MUST resolve `<name>` to a URL by looking it up in the named
registry. The project-local registry `.steer/marketplaces.toml` MUST be consulted
first; if the name is absent there, the user-global registry
`~/.steer/marketplaces.toml` MUST be consulted. The first match wins, so a
project-local entry overrides a user-global entry with the same name.

#### Scenario: a name present in the project-local registry is used

- **WHEN** `.steer/marketplaces.toml` contains `[marketplaces.official]` with
  `url = "https://github.com/wangchen7722/steer-marketplace"` and the user runs
  `steer workflow install --marketplace official`
- **THEN** the installer clones that URL.

#### Scenario: a name absent locally but present globally is used

- **WHEN** `--marketplace community` is passed, `.steer/marketplaces.toml` has no
  `community` entry, and `~/.steer/marketplaces.toml` defines `community`
- **THEN** the installer clones the URL from the user-global registry.

#### Scenario: a project-local entry overrides a user-global entry of the same name

- **WHEN** the name `official` is defined in both the project-local and the
  user-global registry with different URLs
- **THEN** the installer uses the URL from the project-local registry.

#### Scenario: an unknown name is an error before any clone

- **WHEN** `--marketplace nosuch` is passed and no registry defines `nosuch`
- **THEN** the installer prints an error naming the missing entry, performs no
  network access, and exits with a non-zero status.

---

### Requirement: With no `--marketplace`, the URL comes from the env var

When `steer workflow install` is run with no `--marketplace` flag, the installer
MUST read the marketplace URL from the `STEER_MARKETPLACE_URL` environment
variable.

#### Scenario: the env var supplies the default source

- **WHEN** `STEER_MARKETPLACE_URL=https://github.com/foo/bar` is set in the
  environment and the user runs `steer workflow install` with no `--marketplace`
- **THEN** the installer clones `https://github.com/foo/bar`.

#### Scenario: an empty env var counts as unset

- **WHEN** `STEER_MARKETPLACE_URL` is set to an empty string and no
  `--marketplace` is passed
- **THEN** the installer behaves as if the variable were unset (see the
  "no source configured" requirement).

---

### Requirement: No configured source is a fatal, pre-network error

When no `--marketplace` flag is given and `STEER_MARKETPLACE_URL` is unset/empty,
the installer MUST fail before any network access: it MUST print a guidance
message explaining how to configure a source (set the env var, pass
`--marketplace <url>`, or register a named marketplace in
`.steer/marketplaces.toml`) and exit with a non-zero status.

#### Scenario: nothing configured yields guidance and a non-zero exit

- **WHEN** `STEER_MARKETPLACE_URL` is unset and the user runs
  `steer workflow install` with no `--marketplace`
- **THEN** no clone is attempted, a guidance message is printed, and the process
  exits non-zero.

---

### Requirement: The registry is a TOML file of named marketplace URLs

The registry file (`.steer/marketplaces.toml` project-local and/or
`~/.steer/marketplaces.toml` user-global) MUST be a TOML document mapping names
to URLs under a `[marketplaces.<name>]` table, each with a `url` key. A missing
registry file MUST NOT be an error (it simply contributes no named entries). A
registry file that exists but is not valid TOML MUST be a fatal error with a
message naming the file, so a typo does not silently disable a marketplace.

#### Scenario: a well-formed registry contributes its named entries

- **WHEN** `.steer/marketplaces.toml` contains valid `[marketplaces.official]`
  and `[marketplaces.community]` tables
- **THEN** both `official` and `community` resolve via `--marketplace`.

#### Scenario: an absent registry file is tolerated

- **WHEN** neither `.steer/marketplaces.toml` nor `~/.steer/marketplaces.toml`
  exists and the user runs `steer workflow install --marketplace official`
- **THEN** the installer reports the unknown name (per the unknown-name
  requirement) rather than erroring on the missing file.

#### Scenario: a malformed registry file is a fatal error

- **WHEN** `.steer/marketplaces.toml` exists but is not valid TOML
- **THEN** the installer prints an error naming that file and exits non-zero,
  without performing a clone.
