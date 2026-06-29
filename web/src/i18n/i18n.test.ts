import { describe, expect, test } from 'bun:test';
import ts from 'typescript';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';

import { localizeDom, observeLocalization, translateString } from './domLocalizer';
import { enUS, zhCN } from './messages';
import { normalizeLocale } from './I18nContext';

const i18nContextSource = readFileSync(new URL('./I18nContext.tsx', import.meta.url), 'utf8');
const messagesSource = readFileSync(new URL('./messages.ts', import.meta.url), 'utf8');

const i18nDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(i18nDir, '..');

function collectTsxFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    const stat = statSync(path);
    if (stat.isDirectory()) return name === '__tests__' ? [] : collectTsxFiles(path);
    return path.endsWith('.tsx') ? [path] : [];
  });
}


function collectSourceFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    const stat = statSync(path);
    if (stat.isDirectory()) return name === '__tests__' ? [] : collectSourceFiles(path);
    if (!/\.(ts|tsx)$/.test(path) || /\.test\.(ts|tsx)$/.test(path)) return [];
    if (path.includes('/i18n/locales/') || path.endsWith('/i18n/messages.ts')) return [];
    return [path];
  });
}

const chineseLiteralAllowList = new Set([
  'job_instance.运行中',
  'job_instance.成功',
  'job_instance.失败',
  'job_instance.部分失败',
  'job_instance.取消',
  'job_instance.重试中',
  'job_instance.重试耗尽',
  'job_instance.无可用执行节点',
  'job_instance.脚本治理失败',
]);

function hasChinese(value: string): boolean {
  return /[\u4e00-\u9fff]/.test(value);
}

function isIgnoredChineseLiteral(value: string): boolean {
  const normalized = value.replace(/\s+/g, ' ').trim();
  if (!normalized || !hasChinese(normalized)) return true;
  if (chineseLiteralAllowList.has(normalized)) return true;
  if (normalized.includes('${')) return true;
  if (/^https?:\/\//.test(normalized) || /^\/api\//.test(normalized)) return true;
  return false;
}

const visibleAttributePatterns: RegExp[] = [
  /\b(?:title|label|placeholder|extra|message|description|emptyText|aria-label)\s*=\s*"([^"]+)"/g,
  /\b(?:title|label|placeholder|extra|message|description|emptyText)\s*:\s*['"]([^'"]+)['"]/g,
];

const exampleLiteralPatterns = [
  /^admin@example\.com$/,
  /^user@example\.com$/,
  /^https?:\/\//,
  /^sqlite:\/\//,
  /^SELECT\b/,
  /^sha256:/,
  /^[A-Z0-9_]+$/,
  /^\d+[hm]?$/,
  /^\d{4}-\d{2}-\d{2}T/,
  /^\d+\/\d+/,
  /^[a-z]+,[a-z]+$/,
  /^[a-z]+=[^,]+/,
  /^[a-z-]+ \/ [a-z-]+$/,
  /^[a-z]+,-/,
  /^kv\/data\//,
  /^registry\./,
  /^prod\//,
  /^\{\}$/,
  /^cn \/ /,
  /^billing failed jobs$/,
  /^Asia\/Shanghai$/,
  /^\d{2}:\d{2}-\d{2}:\d{2}\//,
  /^prod \/ /,
  /^aws-secrets-manager \/ /,
];

function isExampleLiteral(value: string): boolean {
  return exampleLiteralPatterns.some((pattern) => pattern.test(value));
}

describe('i18n message dictionaries', () => {
  test('has no blank or mechanically broken English translations', () => {
    const brokenPatterns = [/Unifiedconsole/, /Againenable/, /Confirmdisable/, /Roll backrelease/, /G RPC/, /Scriptis/, /Createnew/, /complete Key copy/, /selectplugin/, /showplaintext/, /avoidmisleading/, /pagecreate/, /Taskcreate/, /Conditionbroadcast/, /Loadschedule/, /permission(edit|delete|create)/, /approvalpass/, /expressionpass/, /taskbelonging/, /listunified/, /defaultuse/, /Allnew/, /^$/];
    for (const [source, translated] of Object.entries(enUS)) {
      if (!/[\u4e00-\u9fff]/.test(source)) continue;
      expect(translated.trim(), `empty English translation for ${source}`).not.toBe('');
      expect(/[\u4e00-\u9fff]/.test(translated), `English translation must not contain Chinese for ${source}: ${translated}`).toBe(false);
      for (const pattern of brokenPatterns) {
        expect(pattern.test(translated), `broken English translation for ${source}: ${translated}`).toBe(false);
      }
    }
    expect(enUS['本页分为两栏：Service Account 是 app 作用域机器身份；API-Key 是绑定到 Service Account 的访问凭证。先维护机器身份，再给它签发一个或多个 Key。']).toBe('This page has two sections: Service Account is the machine identity scoped to an app; API-Key is the access credential bound to a Service Account. Maintain the machine identity first, then issue one or more keys for it.');
    expect(enUS['回退草稿']).toBe('Revert to draft');
    expect(enUS['沙箱后端']).toBe('Sandbox backend');
  });

  test('keeps locale dictionaries split into standalone language files', () => {
    expect(messagesSource).toContain('./locales/zh-CN');
    expect(messagesSource).toContain('./locales/en-US');
  });

  test('keeps Chinese and English dictionaries aligned for current UI copy', () => {
    expect(Object.keys(enUS).sort()).toEqual(Object.keys(zhCN).sort());
    expect(enUS['审计日志']).toBe('Audit logs');
    expect(enUS['总览']).toBe('Overview');
    expect(zhCN['审计日志']).toBe('审计日志');
    expect(zhCN['Service Account']).toBe('服务账号');
    expect(zhCN['API-Key']).toBe('接口密钥');
  });

  test('covers visible table headers, enum labels, statuses, and placeholders', () => {
    const requiredTerms = [
      'Name',
      'Status',
      'Actions',
      'Created At',
      'Updated At',
      'Pending',
      'Running',
      'Failed',
      'Daily Time Interval',
      'Processor Type',
      'Channel Type',
      'Grant URL',
      'Namespace scope',
      'pending',
      'dispatching',
      'success',
      'broadcast',
      'fixed_rate',
      '选择作用域管理中的 Namespace',
      '失败重试策略',
      '执行结果',
      '节点执行结果',
      '暂无执行节点信息',
      '广播节点结果',
      '单节点结果',
      '等待 Worker 返回结果',
      'Updated',
      'Logs',
      'Message',
      '执行日志',
      '广播子执行',
      '任务编排',
    ];

    for (const term of requiredTerms) {
      expect(zhCN[term], `missing zh-CN term: ${term}`).toBeDefined();
      expect(enUS[term], `missing en-US term: ${term}`).toBeDefined();
    }

    expect(zhCN['Name']).toBe('名称');
    expect(zhCN['Status']).toBe('状态');
    expect(zhCN['dispatching']).toBe('分发中');
    expect(zhCN['fixed_rate']).toBe('固定频率');
    expect(enUS['执行结果']).toBe('Execution result');
    expect(enUS['节点执行结果']).toBe('Node execution results');
    expect(enUS['广播节点结果']).toBe('Broadcast node results');
    expect(enUS['等待 Worker 返回结果']).toBe('Waiting for Worker result');
  });


  test('keeps Chinese source string literals covered by the i18n dictionaries', () => {
    const files = collectSourceFiles(srcDir);
    const keys = new Set([...Object.keys(zhCN), ...Object.keys(enUS)]);
    const missing: string[] = [];

    for (const file of files) {
      const sourceText = readFileSync(file, 'utf8');
      const sourceFile = ts.createSourceFile(
        file,
        sourceText,
        ts.ScriptTarget.Latest,
        true,
        file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
      );

      const check = (node: ts.Node, value: string) => {
        const normalized = value.replace(/\s+/g, ' ').trim();
        if (isIgnoredChineseLiteral(normalized) || keys.has(normalized)) return;
        const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
        missing.push(`${file.replace(`${srcDir}/`, '')}:${line}:${normalized}`);
      };

      const visit = (node: ts.Node) => {
        if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
          check(node, node.text);
        } else if (ts.isJsxText(node)) {
          check(node, node.getText(sourceFile));
        }
        ts.forEachChild(node, visit);
      };

      visit(sourceFile);
    }

    expect(missing).toEqual([]);
  });

  test('keeps visible JSX and table metadata strings covered by the i18n dictionaries', () => {
    const files = [
      ...collectTsxFiles(join(srcDir, 'pages')),
      ...collectTsxFiles(join(srcDir, 'components')),
      join(srcDir, 'routes.tsx'),
    ];
    const keys = new Set([...Object.keys(zhCN), ...Object.keys(enUS)]);
    const missing: string[] = [];

    for (const file of files) {
      const source = readFileSync(file, 'utf8');
      for (const pattern of visibleAttributePatterns) {
        for (const match of source.matchAll(pattern)) {
          const value = match[1].trim();
          if (!value || value.includes('${') || keys.has(value) || isExampleLiteral(value)) continue;
          if (/^[a-z][a-zA-Z0-9_.-]*$/.test(value)) continue;
          const line = source.slice(0, match.index).split('\n').length;
          missing.push(`${file.replace(`${srcDir}/`, '')}:${line}:${value}`);
        }
      }
    }

    expect(missing).toEqual([]);
  });

  test('observes document body so Ant Design drawer and modal portals are localized', () => {
    expect(i18nContextSource).toContain('const root = document.body');
    expect(i18nContextSource).not.toContain("document.getElementById('root')");
  });

  test('translates exact and embedded string content without changing Chinese default', () => {
    expect(translateString('审计日志', enUS, true)).toBe('Audit logs');
    expect(translateString('已导出 12 条审计记录', enUS, true)).toBe('Exported 12 audit records');
    expect(translateString('刷 新', enUS, true)).toBe('Refresh');
    expect(translateString('审计日志', enUS, false)).toBe('审计日志');
    expect(translateString('Service Account 已创建', zhCN, true)).toBe('服务账号已创建');
    expect(translateString('Worker 集群', zhCN, true)).toBe('执行节点集群');
    expect(translateString('节点执行结果', enUS, true)).toBe('Node execution results');
    expect(translateString('等待 Worker 返回结果', enUS, true)).toBe('Waiting for Worker result');
    expect(translateString('Running', zhCN, true)).toBe('运行中');
    expect(translateString('Worker running failed success Message', zhCN, true)).toBe('Worker running failed success Message');
    expect(translateString('worker finished with failed status', zhCN, true)).toBe('worker finished with failed status');
  });


  test('does not localize machine identifiers or canonical event names', () => {
    for (const token of ['job_instance.running', 'job_instance.failed', 'job_instance.retry_scheduled', 'eventTypes', 'notification-policy-preview']) {
      expect(translateString(token, zhCN, true)).toBe(token);
      expect(translateString(token, enUS, true)).toBe(token);
    }
  });

  test('normalizes unsupported locales to a supported locale', () => {
    expect(normalizeLocale('zh-CN')).toBe('zh-CN');
    expect(normalizeLocale('en-US')).toBe('en-US');
    expect(normalizeLocale('fr-FR')).toMatch(/^(zh-CN|en-US)$/);
  });
});

describe('DOM localizer', () => {




  test('does not self-trigger an endless localization mutation loop', async () => {
    const dom = new JSDOM('<main><button title="退出">审计日志</button></main>');
    globalThis.document = dom.window.document;
    globalThis.NodeFilter = dom.window.NodeFilter;
    globalThis.Node = dom.window.Node;
    globalThis.MutationObserver = dom.window.MutationObserver;

    const root = dom.window.document.querySelector('main')!;
    let mutationCount = 0;
    const externalObserver = new dom.window.MutationObserver((mutations: MutationRecord[]) => {
      mutationCount += mutations.length;
    });
    externalObserver.observe(root, { childList: true, subtree: true, characterData: true, attributes: true });

    const observer = observeLocalization(root, enUS, true);
    localizeDom(root, enUS, true);
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(root.textContent).toBe('Audit logs');
    expect(root.querySelector('button')?.getAttribute('title')).toBe('Sign out');
    expect(mutationCount).toBeLessThan(8);

    observer.disconnect();
    externalObserver.disconnect();
  });

  test('treats later React-style text and attribute changes as new source copy', () => {
    const dom = new JSDOM('<main><button title="退出">审计日志</button></main>');
    globalThis.document = dom.window.document;
    globalThis.NodeFilter = dom.window.NodeFilter;
    globalThis.Node = dom.window.Node;

    const root = dom.window.document.querySelector('main')!;
    const button = root.querySelector('button')!;
    const textNode = button.firstChild as Text;

    localizeDom(root, enUS, true);
    expect(button.textContent).toBe('Audit logs');
    expect(button.getAttribute('title')).toBe('Sign out');

    textNode.nodeValue = '总览';
    button.setAttribute('title', '选择语言');
    localizeDom(root, enUS, true);

    expect(button.textContent).toBe('Overview');
    expect(button.getAttribute('title')).toBe('Choose language');

    localizeDom(root, enUS, false);
    expect(button.textContent).toBe('总览');
    expect(button.getAttribute('title')).toBe('选择语言');
  });



  test('normalizes mixed Chinese-English UI copy in Chinese locale', () => {
    const dom = new JSDOM('<main><h1>SDK Management API-Key</h1><p>Service Account 是 app 作用域机器身份；API-Key 是绑定到 Service Account 的访问凭证。</p><button title="Worker 集群">在线 Worker</button><input placeholder="Namespace" /></main>');
    globalThis.document = dom.window.document;
    globalThis.NodeFilter = dom.window.NodeFilter;
    globalThis.Node = dom.window.Node;

    const root = dom.window.document.querySelector('main')!;
    localizeDom(root, zhCN, true);

    expect(root.textContent).toContain('软件开发工具包接口密钥管理');
    expect(root.textContent).toContain('服务账号 是 应用 作用域机器身份；接口密钥 是绑定到 服务账号 的访问凭证。');
    expect(root.textContent).toContain('在线执行节点');
    expect(root.querySelector('button')?.getAttribute('title')).toBe('执行节点集群');
    expect(root.querySelector('input')?.getAttribute('placeholder')).toBe('命名空间');
  });


  test('does not localize runtime data regions such as logs payloads and returned messages', () => {
    const dom = new JSDOM('<main><h1>执行日志</h1><div role="log"><span>Worker running failed success Message</span></div><p data-runtime-text>Job running failed success Message</p><pre class="json-preview">{"status":"running","message":"Worker failed"}</pre><p class="instance-result-panel__message-body" title="Worker running failed">Worker running failed</p><button title="执行日志">执行日志</button></main>');
    globalThis.document = dom.window.document;
    globalThis.NodeFilter = dom.window.NodeFilter;
    globalThis.Node = dom.window.Node;

    const root = dom.window.document.querySelector('main')!;
    localizeDom(root, zhCN, true);

    expect(root.querySelector('h1')?.textContent).toBe('执行日志');
    expect(root.querySelector('[role="log"]')?.textContent).toBe('Worker running failed success Message');
    expect(root.querySelector('[data-runtime-text]')?.textContent).toBe('Job running failed success Message');
    expect(root.querySelector('.json-preview')?.textContent).toBe('{"status":"running","message":"Worker failed"}');
    expect(root.querySelector('.instance-result-panel__message-body')?.textContent).toBe('Worker running failed');
    expect(root.querySelector('.instance-result-panel__message-body')?.getAttribute('title')).toBe('Worker running failed');
    expect(root.querySelector('button')?.getAttribute('title')).toBe('执行日志');
  });

  test('localizes text and attributes, then restores original Chinese copy', () => {
    const dom = new JSDOM('<main><button aria-label="选择语言" title="退出">审计日志</button><input placeholder="搜索脚本/创建人" /></main>');
    globalThis.document = dom.window.document;
    globalThis.NodeFilter = dom.window.NodeFilter;
    globalThis.Node = dom.window.Node;

    const root = dom.window.document.querySelector('main')!;
    localizeDom(root, enUS, true);

    expect(root.textContent).toContain('Audit logs');
    expect(root.querySelector('button')?.getAttribute('aria-label')).toBe('Choose language');
    expect(root.querySelector('button')?.getAttribute('title')).toBe('Sign out');
    expect(root.querySelector('input')?.getAttribute('placeholder')).toBe('Search scripts / creators');

    localizeDom(root, enUS, false);

    expect(root.textContent).toContain('审计日志');
    expect(root.querySelector('button')?.getAttribute('aria-label')).toBe('选择语言');
    expect(root.querySelector('button')?.getAttribute('title')).toBe('退出');
    expect(root.querySelector('input')?.getAttribute('placeholder')).toBe('搜索脚本/创建人');
  });
});
