import { QuestionCircleOutlined } from '@ant-design/icons';
import { Popover, Space, Table, Tag, Tooltip, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';

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

export function templateVariableRows(variables: string[], t: (value: string) => string): TemplateVariableDefinition[] {
  return Array.from(new Set(variables.map(normalizePlaceholder))).map((placeholder) => {
    const item = VARIABLES.get(placeholder) ?? unknownDefinition(placeholder);
    return {
      placeholder: item.placeholder,
      label: t(item.label),
      description: t(item.description),
      example: item.example,
      source: t(item.source),
    };
  });
}

interface TemplateVariableCatalogProps {
  variables: string[];
  title?: string;
  compact?: boolean;
  t: (value: string) => string;
}

export function TemplateVariableCatalog({ variables, title = '可用模板变量', compact = false, t }: TemplateVariableCatalogProps) {
  const rows = templateVariableRows(variables, t);
  const columns: ColumnsType<TemplateVariableDefinition> = [
    { title: t('占位符'), dataIndex: 'placeholder', key: 'placeholder', width: 150, render: (value: string) => <Typography.Text code copyable>{value}</Typography.Text> },
    { title: t('中文含义'), dataIndex: 'label', key: 'label', width: 120 },
    { title: t('说明'), dataIndex: 'description', key: 'description' },
    { title: t('示例值'), dataIndex: 'example', key: 'example', width: 180, render: (value: string) => <Typography.Text type="secondary">{value}</Typography.Text> },
    { title: t('来源/备注'), dataIndex: 'source', key: 'source', width: 170, render: (value: string) => <Tag>{value}</Tag> },
  ];
  const content = (
    <div className="template-variable-catalog__popover">
      <Typography.Paragraph type="secondary" className="template-variable-catalog__note">
        {t('变量由消息标准字段与事件 payload 顶层字段共同提供；占位符保持英文，展示名称和说明会随界面语言切换。')}
      </Typography.Paragraph>
      <Table size="small" rowKey="placeholder" pagination={false} columns={columns} dataSource={rows} scroll={{ x: 760 }} />
    </div>
  );

  return (
    <Space direction="vertical" size={compact ? 8 : 10} className="template-variable-catalog" style={{ width: '100%' }}>
      <Space size={6} align="center" className="template-variable-catalog__title">
        <Typography.Text strong>{t(title)}</Typography.Text>
        <Popover trigger={["hover", "click"]} placement="leftTop" title={t('变量映射表')} content={content} overlayClassName="available-template-variables">
          <Tooltip title={t('查看变量映射表')}>
            <QuestionCircleOutlined aria-label={t('查看变量映射表')} className="template-variable-catalog__help" />
          </Tooltip>
        </Popover>
      </Space>
      <Space wrap size={[6, 6]}>
        {rows.map((item) => (
          <Tag key={item.placeholder} className="template-variable-catalog__tag">
            <Typography.Text code>{item.placeholder}</Typography.Text>
            <Typography.Text type="secondary"> · {item.label}</Typography.Text>
          </Tag>
        ))}
      </Space>
    </Space>
  );
}
