# RedEye AI Engine Architecture

## Overview
The RedEye AI Engine is a multi-tenant, high-performance API gateway and orchestration layer for interacting with Large Language Models (LLMs). It provides enterprise-ready features such as semantic caching, PII compliance redaction, observability, and dynamic LLM proxying.

## Microservices
1. **RedEye Gateway (`redeye_gateway`)**: The primary entry point. Handles rate limiting, authentication, load balancing, proxying, and streams responses back to clients. Exposes the main `/v1/chat/completions` proxy and management metrics.
2. **RedEye Auth (`redeye_auth`)**: Manages tenant workspaces, user registration (with passwords, OTP, OAuth), virtual API keys, upstream provider API keys securely, and issues JWT tokens.
3. **RedEye Cache (`redeye_cache`)**: A Layer-2 Semantic Cache (via gRPC & REST) that detects similar incoming prompts and serves cached completions to drastically reduce latency and LLM costs.
4. **RedEye Compliance (`redeye_compliance`)**: Handles data localized routing based on Data Privacy regulations and scrubs sensitive Personally Identifiable Information (PII) from prompts before they leave the environment.
5. **RedEye Tracer (`redeye_tracer`)**: Telemetry ingestion engine storing trace data, metrics, and compliance audit logs into ClickHouse for analytics and dashboard visualization.

## Request Lifecycle

The diagram below illustrates the exact path a request takes through the RedEye Engine microservices:

```mermaid
sequenceDiagram
    participant Client
    participant Gateway as redeye_gateway
    participant Auth as redeye_auth (Middleware)
    participant Cache as redeye_cache
    participant Compliance as redeye_compliance
    participant LLM as External LLM
    participant Tracer as redeye_tracer

    Client->>Gateway: POST /v1/chat/completions
    
    %% Auth Check
    Gateway->>Auth: Validate API Key / Token
    Auth-->>Gateway: Claims (tenant_id)
    
    %% Cache Check
    Gateway->>Cache: Check Semantic Cache (gRPC/REST)
    alt Cache Hit
        Cache-->>Gateway: Return Cached Response
    else Cache Miss
        %% PII Compliance Check
        Gateway->>Compliance: /v1/compliance/redact
        Compliance-->>Gateway: Sanitized Prompt & Token Map
        
        %% Upstream Execution
        Gateway->>LLM: Forward Sanitized Request
        LLM-->>Gateway: Response Content
        
        %% Background tasks
        Gateway-)Cache: Store new payload in cache
    end
    
    %% Tracing
    Gateway-)Tracer: Async Log Trace & Audit (ClickHouse)
    
    %% Response
    Gateway-->>Client: Final Client Response
```
