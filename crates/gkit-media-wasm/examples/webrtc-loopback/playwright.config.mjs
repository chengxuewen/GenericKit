import { defineConfig } from '@playwright/test';

export default defineConfig({
  use: {
    browserName: 'chromium',
    headless: true,
    launchOptions: {
      args: [
        '--disable-features=WebRtcHideLocalIpsWithMdns',
        '--use-fake-device-for-media-stream',
        '--use-fake-ui-for-media-stream',
        '--allow-loopback-in-peer-connection',
      ],
    },
  },
  timeout: 180000,
});
