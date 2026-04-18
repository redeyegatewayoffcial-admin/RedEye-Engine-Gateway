import { test, expect } from '@playwright/test';
import { AuthPage } from './pages/AuthPage';

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': 'http://localhost:5173',
  'Access-Control-Allow-Credentials': 'true',
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
};

test.describe('Telemetry & Traces Explorer', () => {
  test.beforeEach(async ({ page }) => {
    // Intercept ANY OPTIONS request and return 204 OK for CORS Preflight
    await page.route('**/*', async (route, request) => {
      if (request.method() === 'OPTIONS') {
        return route.fulfill({ status: 204, headers: CORS_HEADERS });
      }
      route.fallback();
    });

    // 1. Mock Auth endpoints to allow fast UI login
    await page.route('**/otp/request', async (route, request) => {
      if (request.method() === 'OPTIONS') return route.fallback();
      await route.fulfill({ status: 200, headers: CORS_HEADERS, contentType: 'application/json', body: '{}' });
    });
    
    await page.route('**/otp/verify', async (route, request) => {
      if (request.method() === 'OPTIONS') return route.fallback();
      await route.fulfill({
        status: 200,
        headers: CORS_HEADERS,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'test-user-1',
          email: 'test@company.com',
          tenant_id: 'tenant-1',
          workspace_name: 'Test',
          onboarding_complete: true,
          account_type: 'individual'
        })
      });
    });

    // 2. Mock SWR endpoints for Traces
    await page.route('**/admin/traces', async (route, request) => {
      if (request.method() === 'OPTIONS') return route.fallback();
      await route.fulfill({
        status: 200,
        headers: CORS_HEADERS,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            traceId: 'trace-12345',
            tenantId: 'tenant-1',
            model: 'meta-llama/llama-3-8b',
            tokens: 450,
            latency: '120ms',
            policy: 'Allowed'
          },
          {
            traceId: 'trace-67890',
            tenantId: 'tenant-1',
            model: 'openai/gpt-4o',
            tokens: 1200,
            latency: '400ms',
            policy: 'Blocked'
          }
        ])
      });
    });

    const authPage = new AuthPage(page);
    await authPage.goto();
    await authPage.login('test@company.com', '123456');
    await page.waitForURL('**/dashboard');

    // 3. Navigate to the traces route
    await page.getByRole('link', { name: 'Trace Explorer' }).first().click();
  });

  test('displays structural telemetry datatable via SWR', async ({ page }) => {
    const tracesTable = page.getByRole('table');
    await expect(tracesTable).toBeVisible();

    const tableRows = tracesTable.getByRole('row');
    await expect(tableRows).not.toHaveCount(0); 

    await expect(page.getByRole('columnheader', { name: /trace id/i })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: /policy result/i })).toBeVisible();

    await expect(page.getByText('trace-12345')).toBeVisible();
    await expect(page.getByText('trace-67890')).toBeVisible();
    await expect(page.getByText('meta-llama/llama-3-8b')).toBeVisible();
  });
});
