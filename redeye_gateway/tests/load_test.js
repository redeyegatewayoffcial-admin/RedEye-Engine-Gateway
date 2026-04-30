import http from 'k6/http';
import { check, sleep } from 'k6';

/**
 * RedEye API Gateway - Principal Performance Benchmark (v2.0)
 * Optimized for local OS loopback stability.
 * 
 * RUN COMMAND:
 * k6 run -e GATEWAY_TOKEN="your_jwt_here" load_test.js
 */
export default function() {
    cacheHitScenario();
}
export const options = {
    // Principal Tip: Enabling connection reuse reduces OS port exhaustion.
    noConnectionReuse: false, 
    
    thresholds: {
        http_req_failed: ['rate<0.05'],   // Allowing 5% for local OS drops
        http_req_duration: ['p(95)<500'], // 95% of requests should be below 500ms
    },
    
    scenarios: {
        // Scenario A: Optimized Cache Hits
        // Reduced rate to 50 iters/s to prevent Windows TCP stack abortion.
        cache_hits: {
            executor: 'constant-arrival-rate',
            rate: 50, 
            timeUnit: '1s',
            duration: '30s',
            preAllocatedVUs: 10,
            maxVUs: 50,
            exec: 'cacheHitScenario',
        },

        // Scenario B: CPU-Bound PII Scanning (Rayon)
        // Tests the multi-core regex performance with 50k token payloads[cite: 3].
        heavy_pii: {
            executor: 'ramping-arrival-rate',
            startRate: 1,
            timeUnit: '1s',
            preAllocatedVUs: 2,
            maxVUs: 10,
            stages: [
                { duration: '20s', target: 5 }, // Ramp up to 5 req/s
                { duration: '20s', target: 5 }, // Sustain
            ],
            exec: 'heavyPiiScenario',
        },

        // Scenario C: Circuit Breaker & Fallback
        // Tests proactive blacklisting and speculative routing logic[cite: 3].
        fallback_trigger: {
            executor: 'per-vu-iterations',
            vus: 5,
            iterations: 20,
            maxDuration: '1m',
            exec: 'fallbackScenario',
        },
    },
};

// Configuration from Environment
const BASE_URL = 'http://localhost:8080';
const TOKEN = __ENV.GATEWAY_TOKEN || 'test_token';

const HEADERS = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${TOKEN}`,
};

// --- SCENARIO A: Cache Hits ---
export function cacheHitScenario() {
    const payload = JSON.stringify({
        model: 'gpt-4o', //[cite: 1]
        messages: [{ role: 'user', content: 'What is the capital of France?' }],
    });

    const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, { headers: HEADERS });
    console.log(`DEBUG: Status=${res.status}, Body=${res.body}`);
    check(res, {
        'status is 200': (r) => r.status === 200,
        'cache hit header present': (r) => r.headers['x-redeye-cache'] !== undefined, //[cite: 3]
    });
    
    sleep(0.1); 
}

// --- SCENARIO B: Heavy PII (Rayon Stress) ---
// Large payload to force multi-core scanning[cite: 3].
const heavyText = 'Safe content block. '.repeat(5000) + '\nMy email is admin@nmmglobal.com\n' + 'More safe content. '.repeat(5000);

export function heavyPiiScenario() {
    const payload = JSON.stringify({
        model: 'gpt-4o', //[cite: 1]
        messages: [{ role: 'user', content: heavyText }],
    });

    const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, { headers: HEADERS });
    
    check(res, {
        'status is 200 or 403': (r) => r.status === 200 || r.status === 403,
    });
    
    sleep(0.5);
}

// --- SCENARIO C: Fallback Trigger ---
export function fallbackScenario() {
    const payload = JSON.stringify({
        model: 'fail-model', // Configured to trigger a failing primary key[cite: 3]
        messages: [{ role: 'user', content: 'Trigger speculative fallback' }],
    });

    const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, { headers: HEADERS });
    
    check(res, {
        'status is 200 (recovered)': (r) => r.status === 200,
        'hot swap header present': (r) => r.headers['x-redeye-hot-swap'] === '1', //[cite: 3]
    });
    
    sleep(1);
}