import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

const vendorChunkGroups = [
  {
    name: 'vendor-react',
    test: /node_modules[\/](react|react-dom|react-router|react-router-dom)[\/]/,
    priority: 40,
  },
  {
    name: 'vendor-rc-picker',
    test: /node_modules[\/]@rc-component[\/]picker[\/]/,
    priority: 39,
  },
  {
    name: 'vendor-rc-table',
    test: /node_modules[\/]@rc-component[\/]table[\/]/,
    priority: 38,
  },
  {
    name: 'vendor-rc-select-menu',
    test: /node_modules[\/]@rc-component[\/](select|menu|tree|tree-select|cascader|trigger|tooltip|dropdown|virtual-list|overflow|util)[\/]/,
    priority: 37,
  },
  {
    name: 'vendor-rc-components',
    test: /node_modules[\/](@rc-component|rc-)[\/]/,
    priority: 36,
  },
  {
    name: 'vendor-ant-design-icons',
    test: /node_modules[\/]@ant-design[\/]icons[\/]/,
    priority: 34,
  },
  {
    name: 'vendor-ant-design-runtime',
    test: /node_modules[\/]@ant-design[\/](colors|cssinjs|cssinjs-utils|fast-color|react-slick)[\/]/,
    priority: 33,
  },
  {
    name: 'vendor-antd-internals',
    test: /node_modules[\/]antd[\/]es[\/](_util|theme|style|config-provider)[\/]/,
    priority: 33,
  },
  {
    name: 'vendor-antd',
    test: /node_modules[\/]antd[\/]es[\/](date-picker|time-picker|calendar|locale)[\/]/,
    priority: 32,
  },
  {
    name: 'vendor-antd-core',
    test: /node_modules[\/]antd[\/]/,
    priority: 31,
  },
  {
    name: 'vendor-codemirror',
    test: /node_modules[\/](@codemirror|codemirror|@lezer)[\/]/,
    priority: 30,
    maxSize: 260 * 1024,
  },
  {
    name: 'vendor-visual',
    test: /node_modules[\/](lucide-react)[\/]/,
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
