import { QuestionCircleOutlined } from '@ant-design/icons';
import { Button, Empty, Input, Modal, Segmented, Space, Tag, Tooltip, Typography } from 'antd';
import { useMemo, useState } from 'react';

import { PAYLOAD_TEMPLATE_VARIABLES, STANDARD_TEMPLATE_VARIABLES, type TemplateVariableDefinition } from './templateVariableDefinitions';

const VARIABLES = new Map([...STANDARD_TEMPLATE_VARIABLES, ...PAYLOAD_TEMPLATE_VARIABLES].map((item) => [item.placeholder, item]));

function normalizePlaceholder(value: string): string {
  const trimmed = value.trim();
  return trimmed.startsWith('{{') ? trimmed : `{{${trimmed}}}`;
}

function unknownDefinition(placeholder: string): TemplateVariableDefinition {
  return {
    placeholder,
    label: '自定义变量',
    description: '该变量来自提供方 metadata、插件或自定义事件 payload；请确认发送事件时包含同名顶层字段。',
    example: '-',
    source: '提供方 metadata / 插件字段',
  };
}

export type TemplateVariableGroup = 'standard' | 'payload' | 'custom';

type TemplateVariableFilter = TemplateVariableGroup | 'all';

export interface TemplateVariableRow extends TemplateVariableDefinition {
  group: TemplateVariableGroup;
}

function variableGroup(placeholder: string): TemplateVariableGroup {
  if (STANDARD_TEMPLATE_VARIABLES.some((item) => item.placeholder === placeholder)) return 'standard';
  if (PAYLOAD_TEMPLATE_VARIABLES.some((item) => item.placeholder === placeholder)) return 'payload';
  return 'custom';
}

export function templateVariableRows(variables: string[], t: (value: string) => string): TemplateVariableRow[] {
  return Array.from(new Set(variables.map(normalizePlaceholder))).map((placeholder) => {
    const item = VARIABLES.get(placeholder) ?? unknownDefinition(placeholder);
    return {
      placeholder: item.placeholder,
      label: t(item.label),
      description: t(item.description),
      example: item.example,
      source: t(item.source),
      group: variableGroup(item.placeholder),
    };
  });
}

interface TemplateVariableCatalogProps {
  variables: string[];
  title?: string;
  compact?: boolean;
  t: (value: string) => string;
}

function groupTitle(group: TemplateVariableGroup): string {
  if (group === 'standard') return '标准字段';
  if (group === 'payload') return '任务上下文';
  return '自定义';
}

function groupDescription(group: TemplateVariableGroup): string {
  if (group === 'standard') return '通知中心在创建消息时稳定提供，适合标题、级别、时间和资源标识。';
  if (group === 'payload') return '由任务实例、触发来源、操作人和日志透传上下文提供。';
  return '来自 provider metadata、插件或调用方自定义 payload，需要发送事件时显式携带。';
}

function VariableCard({ row, t }: { row: TemplateVariableRow; t: (value: string) => string }) {
  return (
    <article className={`template-variable-card template-variable-card--${row.group}`}>
      <div className="template-variable-card__head">
        <div className="template-variable-card__placeholder"><Typography.Text code copyable>{row.placeholder}</Typography.Text></div>
        <Tag>{row.source}</Tag>
      </div>
      <div className="template-variable-card__body">
        <Typography.Text strong className="template-variable-card__label">{row.label}</Typography.Text>
        <Typography.Paragraph type="secondary">{row.description}</Typography.Paragraph>
      </div>
      <div className="template-variable-card__example">
        <Typography.Text type="secondary">{t('示例值')}</Typography.Text>
        <Typography.Text>{row.example}</Typography.Text>
      </div>
    </article>
  );
}

export function TemplateVariableCatalog({ variables, title = '可用模板变量', compact = false, t }: TemplateVariableCatalogProps) {
  const rows = useMemo(() => templateVariableRows(variables, t), [t, variables]);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [activeGroup, setActiveGroup] = useState<TemplateVariableFilter>('all');
  const previewRows = rows;
  const groupCounts = useMemo(() => ({
    all: rows.length,
    standard: rows.filter((row) => row.group === 'standard').length,
    payload: rows.filter((row) => row.group === 'payload').length,
    custom: rows.filter((row) => row.group === 'custom').length,
  }), [rows]);
  const filteredRows = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    return rows.filter((row) => {
      const matchesGroup = activeGroup === 'all' || row.group === activeGroup;
      const matchesKeyword = !keyword || [row.placeholder, row.label, row.description, row.source, row.example].some((value) => value.toLowerCase().includes(keyword));
      return matchesGroup && matchesKeyword;
    });
  }, [activeGroup, query, rows]);
  const groupedRows = (['standard', 'payload', 'custom'] as const)
    .map((group) => ({ group, rows: filteredRows.filter((row) => row.group === group) }))
    .filter((item) => item.rows.length > 0);
  const filterOptions = [
    { label: `${t('全部变量')} ${groupCounts.all}`, value: 'all' },
    { label: `${t('标准字段')} ${groupCounts.standard}`, value: 'standard' },
    { label: `${t('任务上下文')} ${groupCounts.payload}`, value: 'payload' },
    { label: `${t('自定义')} ${groupCounts.custom}`, value: 'custom' },
  ];

  return (
    <>
      <Space orientation="vertical" size={compact ? 8 : 10} className="template-variable-catalog" style={{ width: '100%' }}>
        <div className="template-variable-catalog__title">
          <Space size={6} align="center">
            <Typography.Text strong>{t(title)}</Typography.Text>
            <Tooltip title={t('查看变量映射表')}>
              <QuestionCircleOutlined aria-label={t('查看变量映射表')} className="template-variable-catalog__help" onClick={() => setOpen(true)} />
            </Tooltip>
          </Space>
          <Button type="link" size="small" onClick={() => setOpen(true)}>{t('变量映射表')}</Button>
        </div>
        <div className="template-variable-catalog__preview" aria-label={t('可用模板变量')}>
          {previewRows.map((item) => (
            <button type="button" key={item.placeholder} className="template-variable-catalog__chip" onClick={() => setOpen(true)} aria-label={`${t('查看变量映射表')} ${item.placeholder}`}>
              <Typography.Text code>{item.placeholder}</Typography.Text>
              <Typography.Text>{item.label}</Typography.Text>
            </button>
          ))}
        </div>
      </Space>
      <Modal
        open={open}
        title={t('变量映射表')}
        footer={null}
        width="min(1120px, calc(100vw - 32px))"
        onCancel={() => setOpen(false)}
        destroyOnClose
        zIndex={1400}
      >
        <div className="template-variable-catalog__modal">
          <div className="template-variable-catalog__toolbar">
            <Typography.Paragraph type="secondary" className="template-variable-catalog__note">
              {t('变量由消息标准字段与事件 payload 顶层字段共同提供；占位符保持英文，展示名称和说明会随界面语言切换。')}
            </Typography.Paragraph>
            <div className="template-variable-catalog__filters">
              <Input.Search allowClear placeholder={t('搜索变量')} value={query} onChange={(event) => setQuery(event.target.value)} />
              <Segmented value={activeGroup} options={filterOptions} onChange={(value) => setActiveGroup(value as TemplateVariableFilter)} />
            </div>
          </div>
          <div className="template-variable-catalog__content">
            {groupedRows.length > 0 ? groupedRows.map(({ group, rows: groupRows }) => (
              <section className="template-variable-catalog__group" key={group}>
                <div className="template-variable-catalog__group-title">
                  <div>
                    <Typography.Text strong>{t(groupTitle(group))}</Typography.Text>
                    <Typography.Paragraph type="secondary">{t(groupDescription(group))}</Typography.Paragraph>
                  </div>
                  <Tag>{groupRows.length}</Tag>
                </div>
                <div className="template-variable-catalog__grid">
                  {groupRows.map((row) => <VariableCard key={row.placeholder} row={row} t={t} />)}
                </div>
              </section>
            )) : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t('未找到变量')} />}
          </div>
        </div>
      </Modal>
    </>
  );
}
