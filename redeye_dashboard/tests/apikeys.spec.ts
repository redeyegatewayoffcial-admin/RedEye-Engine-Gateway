import { test, expect } from '@playwright/test';
import { AuthPage } from './pages/AuthPage';

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': 'http://localhost:5173',
  'Access-Control-Allow-Credentials': 'true',
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
};

test.describe('LLM Provider Management', () => {
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

    // 2. Mock SWR endpoints for API Keys View (Provider Keys)
    await page.route('**/auth/provider-keys', async (route, request) => {
      if (request.method() === 'OPTIONS') return route.fallback();
      if (request.method() === 'GET') {
        await route.fulfill({
          status: 200,
          headers: CORS_HEADERS,
          contentType: 'application/json',
          body: JSON.stringify([
            { id: 'pk_123', provider_name: 'anthropic', created_at: new Date().toISOString() }
          ])
        });
      } else if (request.method() === 'POST') {
        await route.fulfill({ status: 200, headers: CORS_HEADERS, contentType: 'application/json', body: '{}' });
      }
    });

    const authPage = new AuthPage(page);
    await authPage.goto();
    await authPage.login('test@company.com', '123456');
    await page.waitForURL('**/dashboard');

    // 3. Navigate to the API Keys sub-route
    await page.getByRole('link', { name: 'API Keys' }).first().click();
  });

  test('adds a new Provider Vault key via modal', async ({ page }) => {
    const closeTourBtn = page.getByRole('button', { name: 'Close', exact: true });
    if (await closeTourBtn.isVisible()) {
      await closeTourBtn.click();
    }

    const addProviderBtn = page.getByRole('button', { name: /add provider/i });
    await expect(addProviderBtn).toBeVisible();
    await addProviderBtn.click();

    const providerSelect = page.getByRole('combobox');
    const keyInput = page.getByPlaceholder('sk-...');
    const submitBtn = page.getByRole('button', { name: /add provider key/i });

    await expect(keyInput).toBeVisible(); 
    await providerSelect.selectOption('openrouter'); 
    await keyInput.fill('sk-or-v1-playwright-test-key-1234');
    
    await submitBtn.click();
    await expect(keyInput).toBeHidden();
  });
});
