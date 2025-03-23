import { defineConfig } from '@rsbuild/core';
import { pluginVue } from '@rsbuild/plugin-vue';
import { pluginNodePolyfill } from '@rsbuild/plugin-node-polyfill';

export default defineConfig({
  server: {
    port: 1420,
  },
  source: {
    define: {
      'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV),
    },
  },
  output: {
    overrideBrowserslist: ['edge >= 100'],
  },
  performance: {
    chunkSplit: {
      strategy: 'single-vendor',
    },
  },
  plugins: [pluginVue(), pluginNodePolyfill()],
  tools: {
    rspack: {
      experiments: {
        rspackFuture: {
          bundlerInfo: { force: false },
        },
      },
    }
  },
});
