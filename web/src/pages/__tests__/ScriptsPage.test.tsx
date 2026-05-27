import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const scriptsSource = readFileSync(new URL('../ScriptsPage.tsx', import.meta.url), 'utf8');
const codeEditorSource = readFileSync(new URL('../../components/CodeEditor.tsx', import.meta.url), 'utf8');

describe('scripts editor routing', () => {
  test('exposes a secondary script edit route instead of opening the edit modal from the list', () => {
    expect(routesSource).toContain('scriptEdit');
    expect(routesSource).toContain('/scripts/:id/edit');
    expect(appSource).toContain('ScriptEditorPage');
    expect(appSource).toContain('ROUTE_META.scriptEdit.path');
    expect(scriptsSource).not.toContain('openEditModal');
    expect(scriptsSource).not.toContain('编辑脚本 -');
    expect(scriptsSource).toContain('navigate(`/scripts/${record.id}/edit`)');
  });

  test('keeps diff preview confirmation in the secondary editor page', () => {
    expect(scriptsSource).toContain('export function ScriptEditorPage');
    expect(scriptsSource).toContain('变更预览');
    expect(scriptsSource).toContain('确认保存');
    expect(scriptsSource).toContain('更新后将生成新的不可变版本快照');
  });
});

describe('scripts language options', () => {
  test('uses explicit JavaScript and TypeScript language values for editor linting', () => {
    expect(scriptsSource).toContain("{ value: 'javascript', label: 'JavaScript' }");
    expect(scriptsSource).toContain("{ value: 'typescript', label: 'TypeScript' }");
    expect(scriptsSource).not.toContain("{ value: 'js', label: 'JavaScript' }");
    expect(scriptsSource).not.toContain("{ value: 'ts', label: 'TypeScript' }");
  });

  test('keeps JavaScript and TypeScript mapped to distinct CodeMirror parsers', () => {
    expect(codeEditorSource).toContain("case 'javascript':");
    expect(codeEditorSource).toContain('return [javascript(), parseErrorLinter()]');
    expect(codeEditorSource).toContain("case 'typescript':");
    expect(codeEditorSource).toContain('javascript({ typescript: true })');
  });
});
