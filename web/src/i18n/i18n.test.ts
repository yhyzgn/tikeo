import { describe, expect, test } from 'bun:test';
import { JSDOM } from 'jsdom';

import { localizeDom, observeLocalization, translateString } from './domLocalizer';
import { enUS, zhCN } from './messages';
import { normalizeLocale } from './I18nContext';

describe('i18n message dictionaries', () => {
  test('keeps Chinese and English dictionaries aligned for current UI copy', () => {
    expect(Object.keys(enUS).sort()).toEqual(Object.keys(zhCN).sort());
    expect(enUS['审计日志']).toBe('Audit logs');
    expect(enUS['总览']).toBe('Overview');
    expect(zhCN['审计日志']).toBe('审计日志');
    expect(zhCN['Service Account']).toBe('服务账号');
    expect(zhCN['API-Key']).toBe('接口密钥');
  });

  test('translates exact and embedded string content without changing Chinese default', () => {
    expect(translateString('审计日志', enUS, true)).toBe('Audit logs');
    expect(translateString('已导出 12 条审计记录', enUS, true)).toBe('Exported 12 audit records');
    expect(translateString('审计日志', enUS, false)).toBe('审计日志');
    expect(translateString('Service Account 已创建', zhCN, true)).toBe('服务账号已创建');
    expect(translateString('Worker 集群', zhCN, true)).toBe('执行节点集群');
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
