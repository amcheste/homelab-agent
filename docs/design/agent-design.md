# homelab-agent design

**Status:** accepted (phase 1 scaffolded 2026-07-11)
**Owner:** @amcheste

## Context

The homelab is growing a control plane (Go, separate repo) that needs to
know what every node is, whether it is healthy, and eventually how to
change it. The first managed node is the NAS: Rocky Linux 9, UEFI, LVM,
provisioned by [homelab-provisioning](https://github.com/amcheste/homelab-provisioning).
This repo is the per-node agent that reports to that control plane.

The agent is written in Rust. The reasons, in order: a single static
binary with zero runtime dependencies suits a storage box whose real job
is serving files, the idle footprint is a few MB against 50 MB or more
for heavier runtimes, and the same code cross-compiles for the k8s node
and GPU profiles that come later.

## Goals

- Report identity, hardware inventory, storage health, and liveness to
  the control plane with no inbound ports opened on any node.
- Run unprivileged. Phase 1 never mutates the system it observes.
- Survive control plane restarts and network drops without operator
  attention.
- Keep the wire contract in one `.proto` file that the Go control plane
  and the Rust agent both generate from.

## Non-goals (phase 1)

- Actuation of any kind: no disk configuration, no service management,
  no command execution. Phase 2 covers this deliberately (see below).
- Metrics timeseries at scraping resolution. This is fleet state and
  health, not a Prometheus replacement.
- Supporting anything other than Linux.

## Architecture

Three tokio tasks connected by one bounded channel (capacity 256):

```
+-----------+     mpsc      +--------------------+   gRPC bidi stream
| collector | ------------> | connection manager | <==================> control plane
+-----------+               +--------------------+
      |                            reconnects with capped
 sysinfo, smartctl,                exponential backoff (1s..60s)
 statvfs via sysinfo
```

- **Collector** ([src/collect/](../../src/collect/)): gathers a heartbeat
  every 30s, filesystem usage every 5m, and SMART data every 1h
  (all configurable). Inventory is collected at startup and re-sent on
  reconnect. If the channel fills because the control plane is down, the
  newest report is dropped and collection continues; heartbeats are cheap
  and SMART data is re-collectable, so durability buys little here.
- **Connection manager** ([src/transport/](../../src/transport/)): dials
  out to the control plane, holds one persistent bidirectional gRPC
  stream, and forwards collector output. Backoff is exponential from 1s
  capped at 60s.
- **Main** ([src/main.rs](../../src/main.rs)): config, tracing, task
  supervision, signal handling.

### Transport: outbound-only gRPC

The agent always dials out (push model). The control plane never needs a
route to a node, no node opens an inbound port, and NAT or firewall rules
on the node side stay untouched. The stream is bidirectional so that
phase 2 commands ride the same connection with zero transport changes.

The contract lives at
[proto/homelab/agent/v1/agent.proto](../../proto/homelab/agent/v1/agent.proto):

```proto
service AgentGateway {
  rpc Enroll(EnrollRequest) returns (EnrollResponse);
  rpc Stream(stream AgentMessage) returns (stream ControlMessage);
}
```

`AgentMessage` is a oneof over Hello, Heartbeat, Inventory, SmartReport,
and StorageUsage. `ControlMessage` carries only Ack and Ping in phase 1.

Evolution rules: field numbers are never reused, new fields are additive,
and behavior gates on the `agent_version` reported in Hello. The proto
lives in this repo for now; once the control plane API surface grows it
moves to a shared home (likely buf-managed with breaking-change checks
in CI).

The Rust side generates via `tonic-build` with a vendored `protoc`
(hermetic builds, no system protobuf needed). The Go side generates via
`protoc-gen-go` / `protoc-gen-go-grpc` from the same file; the
`go_package` option already points at the control plane's gen path.

### Enrollment and trust

Nodes prove they are ours with a one-time enrollment token:

1. Provisioning (kickstart `%post`) writes a single-use token to
   `/etc/homelab-agent/enrollment-token`, rendered through the existing
   `@KS_*@` secrets flow so no token ever lands in git.
2. On first contact the agent calls `Enroll`, exchanging the token for a
   per-node credential persisted at `/var/lib/homelab-agent/credential`.
3. The credential authenticates every subsequent `Stream` call as request
   metadata. A stolen enrollment token is only good once; a stolen
   credential identifies exactly one node and can be revoked centrally.

TLS is rustls end to end. Server verification uses the system trust store
(or a pinned homelab CA later). Client identity is the credential, not
mTLS, in phase 1; moving to mTLS certs issued at enrollment is a
compatible upgrade if wanted.

Enrollment is scaffolded in the proto but not yet wired in the agent;
the connection manager carries a TODO until the control plane exists to
enroll against.

### What gets reported

| Report | Interval | Source |
| --- | --- | --- |
| Heartbeat (uptime, load, memory) | 30s | sysinfo |
| Storage usage (per filesystem) | 5m | sysinfo |
| SMART health | 1h | `smartctl --json`, relayed verbatim |
| Inventory (CPU, RAM, disks, NICs) | on connect | sysinfo |

SMART reports carry the full `smartctl --json` output as a string next to
3 extracted summary fields (healthy, temperature, power-on hours). The
control plane can deepen its parsing without an agent release.

## Deployment

- **Build target:** `x86_64-unknown-linux-musl`, fully static. Rocky 9
  ships glibc 2.34; a musl build sidesteps glibc versioning entirely.
  TLS is rustls, so no OpenSSL headaches in the static build.
- **Install:** one binary at `/usr/local/bin/homelab-agent`, config at
  `/etc/homelab-agent/config.toml`, systemd unit from
  [systemd/homelab-agent.service](../../systemd/homelab-agent.service).
- **Privileges:** dedicated `homelab-agent` user, `ProtectSystem=strict`,
  `NoNewPrivileges=yes`. The one exception smartctl needs is a sudoers
  fragment allowing exactly `smartctl` (or membership in the `disk`
  group, judged at install time). Nothing else escalates.
- **Runtime dependency:** `smartmontools`, available in the standard
  Rocky 9 repos.
- **Provisioning tie-in:** homelab-provisioning's kickstart `%post` will
  install the binary, unit, config, and enrollment token. First agent
  check-in doubles as the signal that provisioning succeeded.
- **SELinux:** Rocky 9 enforces by default. The unit runs as
  `unconfined_service_t` which is fine for phase 1; if a tighter policy
  comes later, `ausearch -m avc` is the first stop when debugging.

## Phase 2 (sketch, not committed)

Actuation: disk configuration on the NAS first, likely more later. The
transport is already in place (`ControlMessage` grows new payload
variants). What phase 2 must add is the security design, which is the
actual work:

- Per-action authorization, not a blanket "execute" verb.
- An allowlist of typed operations, no free-form shell.
- Audit logging of every command received and its outcome.
- Probably a separate privileged helper process, keeping the agent
  itself unprivileged.

Nothing in phase 1 should need rework for phase 2; if it does, that is a
phase 1 bug.

## Alternatives considered

- **Go agent:** viable and faster to write, but loses the static-binary
  size or the pleasure factor, and the operator wants Rust here.
- **Prometheus node_exporter + scraping:** solves metrics but not
  identity, enrollment, or the phase 2 command channel, and requires the
  control plane to reach into nodes.
- **MQTT/NATS transport:** adds a broker to operate for a two-party
  system. gRPC keeps it broker-free; revisit only if fan-out grows past
  a dozen nodes.
- **Plain HTTPS POST polling:** simplest possible v1 but leaves no live
  channel for phase 2, forcing a transport migration later.

## Open questions

1. Does the control plane terminate TLS itself or sit behind a reverse
   proxy? Affects which cert the agent pins.
2. Credential format: opaque bearer string vs JWT vs mTLS cert at
   enrollment. Phase 1 scaffolding assumes an opaque string.
3. Where the proto's long-term home is once a second consumer appears
   (control plane repo vs dedicated protos repo with buf).
