import { test, expect } from '@playwright/test';
import { AuthPage } from './pages/AuthPage';

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': 'http://localhost:5173',
  'Access-Control-Allow-Credentials': 'true',
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
};

test.describe('Authentication Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Intercept ANY OPTIONS request and return 204 OK for CORS Preflight
    await page.route('**/*', async (route, request) => {
      if (request.method() === 'OPTIONS') {
        return route.fulfill({ status: 204, headers: CORS_HEADERS });
      }
      route.fallback();
    });

    // 1. Intercept OTP Request
    await page.route('**/otp/request', async (route, request) => {
      if (request.method() === 'OPTIONS') return route.fallback(); // Handled strictly above
      await route.fulfill({
        status: 200,
        headers: CORS_HEADERS,
        contentType: 'application/json',
        body: JSON.stringify({ success: true })
      });
    });

    // 2. Intercept OTP Verify
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
          auth_provider: 'email_otp',
          workspace_name: 'Test Workspace',
          onboarding_complete: true,
          account_type: 'individual'
        })
      });
    });
  });

  test('successful magic link login redirects to dashboard metrics', async ({ page }) => {
    const authPage = new AuthPage(page);
    await authPage.goto();

    await authPage.login('test@company.com', '123456');

    // Wait for the route transition
    await page.waitForURL('**/dashboard');
    
    // Verify an element on the dashboard appears
    const dashboardHeading = page.getByRole('heading', { name: /redeye gateway/i });
    await expect(dashboardHeading).toBeVisible();
  });
});
