import { Page, Locator } from '@playwright/test';

export class AuthPage {
  readonly page: Page;
  readonly emailInput: Locator;
  readonly sendCodeButton: Locator;
  readonly otpInput: Locator;
  readonly verifyButton: Locator;

  constructor(page: Page) {
    this.page = page;
    
    // Auth request phase locators
    this.emailInput = page.getByPlaceholder('you@company.com');
    this.sendCodeButton = page.getByRole('button', { name: /send magic code/i });

    // Auth verify phase locators
    this.otpInput = page.getByPlaceholder('000000');
    this.verifyButton = page.getByRole('button', { name: /verify & login/i });
  }

  async goto() {
    await this.page.goto('/login');
  }

  async login(email: string, otp: string = '000000') {
    // 1. Submit email
    await this.emailInput.fill(email);
    await this.sendCodeButton.click();

    // 2. Wait for Framer Motion animation to slide in the OTP field
    // Playwright natively handles this, but it's good to ensure it's visible
    // using the explicit locator
    await this.otpInput.waitFor({ state: 'visible' });

    // 3. Submit OTP
    await this.otpInput.fill(otp);
    await this.verifyButton.click();
  }
}
