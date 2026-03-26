import http from 'k6/http';
import { check, sleep } from 'k6';

// Direct Concurrent Test (No ramp up/down)
export const options = {
    vus: 500,        // EXACT-aa 500 users ore time-la hit pannuvanga
    duration: '30s', // Thodarndhu 30 seconds-ku intha 500 perum parallel-aa adippanga
};

export default function () {
    const url = 'http://localhost:8080/v1/chat/completions';

    const payload = JSON.stringify({
        model: "gpt-4o",
        messages: [{ role: "user", content: "Concurrent spike test!" }]
    });

    const params = {
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer test_token_123',
            'x-tenant-id': 'test-tenant-uuid' 
        },
    };

    const res = http.post(url, payload, params);

    check(res, {
        // Namma expected errors (Unauthorized or Rate Limited) varutha nu check
        'handled correctly (401 or 429)': (r) => r.status === 401 || r.status === 429,
        // Request romba time edukkama fast-aa process aagutha nu check
        'latency under 100ms': (r) => r.timings.duration < 100,
    });

    // 100ms gap vitu adutha request anuppuvanga (More realistic user behavior)
    sleep(0.1); 
}