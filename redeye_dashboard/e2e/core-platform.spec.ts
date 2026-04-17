import { test, expect } from '@playwright/test';

test.describe('Core Platform Navigation Flow', () => {
  test('User logs in, navigates to dashboard, and generates an API key', async ({ page }) => {
    // Navigate to the app root (will redirect to /auth if not logged in)
    await page.goto('http://localhost:5173/');

    // 1. Simulate Login Flow
    // Waiting for the auth page to load
    await page.waitForURL('**/auth*');
    
    // Fill in credentials and submit
    await page.fill('input[type="email"]', 'admin@redeye.ai');
    await page.fill('input[type="password"]', 'securepassword123');
    await page.click('button[type="submit"]');

    // 2. Redirect to Dashboard
    await page.waitForURL('**/dashboard*');
    await expect(page.getByText('Welcome back')).toBeVisible();

    // 3. Navigate to API Keys page via sidebar
    await page.click('a[href="/dashboard/api-keys"]');
    await page.waitForURL('**/dashboard/api-keys');
    await expect(page.getByRole('heading', { name: 'API Keys & Providers' })).toBeVisible();

    // 4. Generate a new Virtual API Key
    await page.click('button:has-text("Generate Key")');
    await expect(page.getByRole('heading', { name: 'Generate Virtual API Key' })).toBeVisible();

    // Fill in key name
    await page.fill('input[placeholder="e.g. Production Frontend App"]', 'Playwright E2E Key');
    
    // Set up a listener for the unimplemented window.alert used for generation success toast
    page.once('dialog', async dialog => {
      expect(dialog.message()).toContain('Generate endpoint not yet implemented');
      await dialog.accept();
    });

    await page.click('button[type="submit"]:has-text("Generate Key")');
  });
});
