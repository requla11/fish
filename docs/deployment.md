# Production Deployment Guide

Comprehensive instructions for deploying Fish in cloud and production CI/CD environments.

> ⚠️ **Status note:** Topologies 2 and 3 below (distributed cluster with
> `fish-coordinator`, Kubernetes autoscaling) describe **planned** services
> that do not exist in this repository yet. Today only single-node local CI
> and the opt-in `fish worker` / `fish cache-server` processes are available.
> See [ARCHITECTURE.md](ARCHITECTURE.md).

## Deployment Topologies
1. **Single-Node Local CI**: Embedded high-speed caching on runner host.
2. **Distributed Cluster**: `fish-coordinator` managing remote worker pools.
3. **Cloud-Native Kubernetes**: Automated autoscaling worker pods with Spot instance lifecycle handling.

## Service Ports
- **Coordinator HTTP/gRPC**: `9090`
- **Worker Gateway**: `9091`
- **P2P Swarm Network**: `7890` (Cache) / `7891` (Compute)
- **OpenTelemetry Metrics**: `9094`
