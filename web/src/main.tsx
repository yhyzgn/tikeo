import React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import 'antd/dist/reset.css';
import './styles.css';
import './components/logs/terminalLogs.css';

import { App } from './App';
import { I18nProvider } from './i18n';

/**
 * Mount the Tikeo management console.
 *
 * BrowserRouter is used so deep links such as /api-keys and /workflows/:id/edit work through the
 * nginx/Vite fallback routes documented in deployment guides.
 */
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <I18nProvider>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </I18nProvider>
  </React.StrictMode>,
);
