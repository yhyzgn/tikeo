import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

const vendorChunkGroups = [
  {
    name: 'vendor-react',
    test: /node_modules[\/](react|react-dom|react-router|react-router-dom)[\/]/,
    priority: 40,
  },
  {
    name: 'vendor-antd',
    test: /node_modules[\/](antd|@ant-design|rc-|@rc-component)[\/]/,
    priority: 30,
    maxSize: 260 * 1024,
  },
  {
    name: 'vendor-codemirror',
    test: /node_modules[\/](@codemirror|codemirror|@lezer)[\/]/,
    priority: 30,
    maxSize: 260 * 1024,
  },
  {
    name: 'vendor-visual',
    test: /node_modules[\/](lucide-react|@ant-design[\/]icons)[\/]/,
    priority: 20,
  },
  {
    name: 'vendor-utils',
    test: /node_modules[\/](diff)[\/]/,
    priority: 10,
  },
];

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
  },
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          minSize: 20 * 1024,
          groups: vendorChunkGroups,
        },
      },
    },
  },
  server: {
    proxy: {
      '^/api(?:/|$)': 'http://0.0.0.0:9090',
      '^/api-docs(?:/|$)': 'http://0.0.0.0:9090',
    },
  },
});
