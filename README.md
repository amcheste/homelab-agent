<!--
Project banner. Spec:
  https://github.com/amcheste/alanchester-brand/blob/main/docs/banner-spec.md

To enable: generate a banner via Claude Design (paste the
design-session-brief plus the banner-spec request prompt), land
the generated SVG and PNG exports in `assets/`, then uncomment the
<img> block below by removing this whole HTML comment block and
restoring the <p> tag.

<p align="center">
  <img src="assets/banner.svg" alt="homelab-agent banner" width="100%">
</p>
-->

<div align="center">

# homelab-agent

**Rust observability agent for homelab nodes, reporting to the Go control plane over gRPC.**

[![Validate](https://github.com/amcheste/homelab-agent/actions/workflows/validate.yml/badge.svg)](https://github.com/amcheste/homelab-agent/actions/workflows/validate.yml)
[![Version](https://img.shields.io/github/v/tag/amcheste/homelab-agent?label=version&sort=semver&color=0B0B0C)](https://github.com/amcheste/homelab-agent/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-1F4D3A.svg)](LICENSE)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/amcheste/homelab-agent/badge)](https://scorecard.dev/viewer/?uri=github.com/amcheste/homelab-agent)

</div>

---

A small agent that runs on every managed homelab node, dials out to the
control plane, and holds one persistent gRPC stream. Phase 1 is
observability only: heartbeat, hardware inventory, filesystem usage, and
SMART health. The agent never opens an inbound port and never mutates
the system it observes.

First target: the NAS on Rocky Linux 9, provisioned by
[homelab-provisioning](https://github.com/amcheste/homelab-provisioning).

## how it works

```
+-----------+     mpsc      +--------------------+   gRPC bidi stream
| collector | ------------> | connection manager | <==================> control plane (Go)
+-----------+               +--------------------+
```

The collector gathers telemetry on configurable intervals (heartbeat
30s, storage 5m, SMART 1h). The connection manager dials the control
plane and reconnects with capped exponential backoff. The wire contract
is a single proto file at
[`proto/homelab/agent/v1/agent.proto`](proto/homelab/agent/v1/agent.proto);
both this repo (tonic) and the Go control plane (protoc-gen-go) generate
from it.

The full design, including enrollment, trust, and the phase 2 actuation
sketch, is in [`docs/design/agent-design.md`](docs/design/agent-design.md).

## building

```sh
cargo build                # dev build; protoc is vendored, no system deps
cargo test
cargo build --release --target x86_64-unknown-linux-musl   # deploy artifact
```

The release artifact is a fully static binary. Rocky 9's glibc version
is irrelevant to it; copy it over and it runs.

No local Rust toolchain? The same builds run in a container:

```sh
docker run --rm -v "$PWD:/work" -w /work rust:1 cargo build
```

## deploying

One binary, one config file, one systemd unit:

| Artifact | Destination |
| --- | --- |
| `homelab-agent` binary | `/usr/local/bin/homelab-agent` |
| [`config/config.example.toml`](config/config.example.toml) | `/etc/homelab-agent/config.toml` |
| [`systemd/homelab-agent.service`](systemd/homelab-agent.service) | `/etc/systemd/system/` |

Runtime dependency: `smartmontools` (standard Rocky 9 repos). The agent
runs as an unprivileged user with systemd hardening; see the design doc
for the smartctl privilege model.

## repo layout

```
proto/      wire contract (source of truth for agent <-> control plane)
src/        agent implementation
  collect/  telemetry gathering (sysinfo, smartctl)
  transport/ gRPC connection management
systemd/    hardened service unit
config/     example configuration
docs/design/ design doc
```
