import {
  DeleteOutlined,
  EditOutlined,
  PlusOutlined,
  ReloadOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Drawer,
  Form,
  Input,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from "antd";
import { useEffect, useState } from "react";

import {
  createPlugin,
  deletePlugin,
  listPlugins,
  updatePlugin,
  type PluginSummary,
} from "../api/client";
import { PermissionGate } from "../components/Permission";

const PLUGIN_KIND_OPTIONS = [
  { value: "mixed", label: "混合插件" },
  { value: "processor", label: "自定义处理器类型" },
  { value: "alert_channel", label: "自定义告警通道" },
];

const PROCESSOR_TYPE_OPTIONS = [
  {
    value: "sql",
    label: "SQL 处理器",
    description: "用于 SQL 同步、ETL、数据治理类任务",
  },
  {
    value: "etl",
    label: "ETL 处理器",
    description: "用于数据抽取/转换/加载任务",
  },
  {
    value: "ai_agent",
    label: "AI Agent 处理器",
    description: "用于智能代理、模型编排类任务",
  },
  {
    value: "ops_action",
    label: "运维动作处理器",
    description: "用于发布、巡检、变更动作任务",
  },
  {
    value: "external_jar",
    label: "外部 JAR/容器处理器",
    description: "用于版本化外部 JAR，由容器镜像在沙箱中承载执行",
  },
];

const ALERT_CHANNEL_OPTIONS = [
  {
    value: "ops_webhook",
    label: "Ops Webhook",
    description: "投递到运维网关或内部告警桥",
  },
  {
    value: "incident_webhook",
    label: "Incident Webhook",
    description: "投递到事故响应系统",
  },
  {
    value: "audit_webhook",
    label: "Audit Webhook",
    description: "投递到审计事件系统",
  },
];

const ALERT_TARGET_KIND_OPTIONS = [{ value: "webhook", label: "Webhook" }];

const ALERT_TEMPLATE_OPTIONS = [
  {
    value: "default_webhook",
    label: "默认文本模板",
    template: {
      body: {
        text: "{{message}}",
        resource: "{{resource_id}}",
        severity: "{{severity}}",
      },
    },
  },
  {
    value: "ops_event",
    label: "运维事件模板",
    template: {
      body: {
        event: "{{message}}",
        resource: "{{resource_type}}/{{resource_id}}",
        level: "{{severity}}",
      },
    },
  },
];

interface PluginFormValues {
  name: string;
  kind: string;
  enabled: boolean;
  processorType?: string;
  processorLabel?: string;
  processorNames?: string[];
  processorDescription?: string;
  artifactRef?: string;
  containerImage?: string;
  entrypoint?: string[];
  checksum?: string;
  alertType?: string;
  alertLabel?: string;
  alertTargetKind?: string;
  alertTemplate?: string;
  alertDescription?: string;
}

export function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<PluginSummary | null>(null);
  const [form] = Form.useForm<PluginFormValues>();
  const selectedProcessorType = Form.useWatch("processorType", form);

  const reload = async () => {
    setLoading(true);
    try {
      setPlugins(await listPlugins());
    } catch (error) {
      message.error(error instanceof Error ? error.message : "加载插件失败");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
  }, []);

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    form.setFieldsValue({
      kind: "mixed",
      enabled: true,
      alertTargetKind: "webhook",
      alertTemplate: "default_webhook",
    });
    setDrawerOpen(true);
  };

  const openEdit = (plugin: PluginSummary) => {
    setEditing(plugin);
    const processor = plugin.processorTypes[0];
    const channel = plugin.alertChannelTypes[0];
    form.setFieldsValue({
      name: plugin.name,
      kind: plugin.kind,
      enabled: plugin.enabled,
      processorType: processor?.type,
      processorLabel: processor?.label,
      processorNames: processor?.processorNames?.length
        ? processor.processorNames
        : undefined,
      processorDescription: processor?.description ?? undefined,
      artifactRef: processor?.artifactRef ?? undefined,
      containerImage: processor?.containerImage ?? undefined,
      entrypoint: processor?.entrypoint ?? undefined,
      checksum: processor?.checksum ?? undefined,
      alertType: channel?.type,
      alertLabel: channel?.label,
      alertTargetKind: channel?.targetKind ?? "webhook",
      alertTemplate: "default_webhook",
      alertDescription: channel?.description ?? undefined,
    });
    setDrawerOpen(true);
  };

  const handleSubmit = async (values: PluginFormValues) => {
    const processorTypes = values.processorType
      ? [
          {
            type: values.processorType,
            label:
              values.processorLabel ||
              PROCESSOR_TYPE_OPTIONS.find(
                (item) => item.value === values.processorType,
              )?.label ||
              values.processorType,
            capability: values.processorType,
            processorNames:
              values.processorNames?.filter((item) => item.trim()) ?? [],
            description: values.processorDescription || null,
            artifactRef: values.processorType === "external_jar" ? values.artifactRef || null : null,
            containerImage: values.processorType === "external_jar" ? values.containerImage || null : null,
            entrypoint: values.processorType === "external_jar" ? values.entrypoint?.filter((item) => item.trim()) ?? [] : null,
            checksum: values.processorType === "external_jar" ? values.checksum || null : null,
          },
        ]
      : [];
    const template =
      ALERT_TEMPLATE_OPTIONS.find((item) => item.value === values.alertTemplate)
        ?.template ?? ALERT_TEMPLATE_OPTIONS[0].template;
    const alertChannelTypes = values.alertType
      ? [
          {
            type: values.alertType,
            label:
              values.alertLabel ||
              ALERT_CHANNEL_OPTIONS.find(
                (item) => item.value === values.alertType,
              )?.label ||
              values.alertType,
            targetKind: values.alertTargetKind || "webhook",
            description: values.alertDescription || null,
            template,
          },
        ]
      : [];
    if (editing) {
      await updatePlugin(editing.id, {
        name: values.name,
        kind: values.kind,
        enabled: values.enabled,
        processorTypes,
        alertChannelTypes,
      });
      message.success("插件已更新");
    } else {
      await createPlugin({
        name: values.name,
        kind: values.kind,
        enabled: values.enabled,
        processorTypes,
        alertChannelTypes,
      });
      message.success("插件已创建");
    }
    setDrawerOpen(false);
    form.resetFields();
    await reload();
  };

  const handleDelete = async (id: string) => {
    await deletePlugin(id);
    message.success("插件已删除");
    await reload();
  };

  return (
    <div className="page-stack plugins-page">
      <section className="hero-panel plugins-hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="purple" className="soft-tag">
              Plugin Registry
            </Tag>
            <Typography.Title level={3}>插件系统</Typography.Title>
          </div>
          <Typography.Text className="hero-panel__desc">
            注册自定义处理器类型与告警通道类型；Worker
            通过结构化插件处理器声明承接任务，告警通道以 webhook 目标闭环投递。
          </Typography.Text>
        </div>
        <div className="hero-panel__actions">
          <Button
            icon={<ReloadOutlined />}
            onClick={() => void reload()}
            loading={loading}
          >
            刷新
          </Button>
          <PermissionGate resource="tenants" action="manage">
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
              注册插件
            </Button>
          </PermissionGate>
        </div>
      </section>

      <Alert
        type="info"
        showIcon
        message="插件能力闭环"
        description="自定义处理器类型会出现在任务创建/编辑的插件处理器选项中；任务保存时校验 processorName 必须来自该类型候选列表，调度时按 Worker 注册的 pluginProcessors.type + processorNames 结构化字段匹配。自定义告警通道用于告警规则投递状态检查，并在 webhook 投递时解析 body/headers 模板。"
      />

      <Card className="clean-card" title="插件注册中心">
        <Table<PluginSummary>
          rowKey="id"
          loading={loading}
          dataSource={plugins}
          scroll={{ x: 980 }}
          columns={[
            {
              title: "名称",
              dataIndex: "name",
              width: 180,
              render: (value: string) => <strong>{value}</strong>,
            },
            {
              title: "类型",
              dataIndex: "kind",
              width: 120,
              render: (value: string) => <Tag>{value}</Tag>,
            },
            {
              title: "自定义处理器类型",
              width: 280,
              render: (_, plugin) => (
                <Space wrap>
                  {plugin.processorTypes.map((item) => (
                    <Tag color={item.type === "external_jar" ? "volcano" : "blue"} key={item.type}>
                      {item.type} · {item.label}{item.artifactRef ? ` · ${item.artifactRef}` : ""}
                    </Tag>
                  ))}
                </Space>
              ),
            },
            {
              title: "自定义告警通道",
              width: 240,
              render: (_, plugin) => (
                <Space wrap>
                  {plugin.alertChannelTypes.map((item) => (
                    <Tag color="purple" key={item.type}>
                      {item.type} · {item.targetKind}
                    </Tag>
                  ))}
                </Space>
              ),
            },
            {
              title: "状态",
              dataIndex: "enabled",
              width: 100,
              render: (enabled: boolean) => (
                <Tag color={enabled ? "green" : "default"}>
                  {enabled ? "enabled" : "disabled"}
                </Tag>
              ),
            },
            { title: "更新时间", dataIndex: "updatedAt", width: 190 },
            {
              title: "操作",
              fixed: "right",
              width: 160,
              render: (_, plugin) => (
                <PermissionGate resource="tenants" action="manage">
                  <Space>
                    <Button
                      size="small"
                      icon={<EditOutlined />}
                      onClick={() => openEdit(plugin)}
                    >
                      编辑
                    </Button>
                    <Button
                      danger
                      size="small"
                      icon={<DeleteOutlined />}
                      onClick={() => void handleDelete(plugin.id)}
                    >
                      删除
                    </Button>
                  </Space>
                </PermissionGate>
              ),
            },
          ]}
        />
      </Card>

      <Drawer
        title={editing ? `编辑插件 - ${editing.name}` : "注册插件"}
        open={drawerOpen}
        width={760}
        destroyOnClose
        onClose={() => {
          setDrawerOpen(false);
          form.resetFields();
        }}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={(values) => void handleSubmit(values)}
        >
          <Form.Item
            name="name"
            label="插件名称"
            rules={[{ required: true, message: "请输入插件名称" }]}
          >
            <Input placeholder="Ops Plugin" />
          </Form.Item>
          <Form.Item name="kind" label="插件分类" rules={[{ required: true }]}>
            <Select options={PLUGIN_KIND_OPTIONS} />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Card
            size="small"
            title="自定义处理器类型"
            className="plugin-form-card"
          >
            <Form.Item name="processorType" label="Processor Type">
              <Select
                allowClear
                showSearch
                placeholder="选择处理器类型"
                options={PROCESSOR_TYPE_OPTIONS.map((item) => ({
                  value: item.value,
                  label: `${item.label} · ${item.value}`,
                }))}
              />
            </Form.Item>
            <Form.Item name="processorLabel" label="展示名">
              <Select
                allowClear
                placeholder="选择展示名或保持自动生成"
                options={PROCESSOR_TYPE_OPTIONS.map((item) => ({
                  value: item.label,
                  label: item.label,
                }))}
              />
            </Form.Item>
            <Form.Item label="Worker 结构化声明">
              <Typography.Text code>
                {selectedProcessorType
                  ? `pluginProcessors.type=${selectedProcessorType}`
                  : "选择 Processor Type 后生成结构化匹配字段"}
              </Typography.Text>
            </Form.Item>
            <Form.Item
              name="processorNames"
              label="任务处理器名候选"
              extra="任务管理页面只会从这里选择 processorName；例如 Java demo 的 billing.sql-sync。"
            >
              <Select
                mode="tags"
                placeholder="输入或选择 Worker 内部处理器名"
                options={[
                  { value: "billing.sql-sync" },
                  { value: "billing.sql-sync.v2" },
                  { value: "ops.sql-sync" },
                ]}
              />
            </Form.Item>
            {selectedProcessorType === "external_jar" ? (
              <>
                <Alert type="warning" showIcon message="外部 JAR 必须通过容器沙箱执行" description="artifactRef 记录版本化 JAR 坐标或对象存储引用，containerImage 指定实际执行镜像；Worker 仍按 pluginProcessors.type=external_jar + processorNames 匹配。" />
                <Form.Item name="artifactRef" label="JAR Artifact Ref" rules={[{ required: true, message: "请输入 JAR artifactRef" }]}>
                  <Input placeholder="s3://tikee-plugins/billing-sync-1.0.0.jar 或 maven:group:artifact:version" />
                </Form.Item>
                <Form.Item name="containerImage" label="Container Image" rules={[{ required: true, message: "请输入容器镜像" }]}>
                  <Input placeholder="registry.example.com/tikee/jar-runner:1.0.0" />
                </Form.Item>
                <Form.Item name="entrypoint" label="Entrypoint">
                  <Select mode="tags" placeholder="java,-jar,/plugins/billing-sync.jar" />
                </Form.Item>
                <Form.Item name="checksum" label="Checksum">
                  <Input placeholder="sha256:..." />
                </Form.Item>
              </>
            ) : null}
            <Form.Item name="processorDescription" label="说明">
              <Input.TextArea rows={2} />
            </Form.Item>
          </Card>
          <Card
            size="small"
            title="自定义告警通道"
            className="plugin-form-card"
          >
            <Form.Item name="alertType" label="Channel Type">
              <Select
                allowClear
                showSearch
                placeholder="选择告警通道类型"
                options={ALERT_CHANNEL_OPTIONS.map((item) => ({
                  value: item.value,
                  label: `${item.label} · ${item.value}`,
                }))}
              />
            </Form.Item>
            <Form.Item name="alertLabel" label="展示名">
              <Select
                allowClear
                placeholder="选择展示名或保持自动生成"
                options={ALERT_CHANNEL_OPTIONS.map((item) => ({
                  value: item.label,
                  label: item.label,
                }))}
              />
            </Form.Item>
            <Form.Item name="alertTargetKind" label="Target Kind">
              <Select options={ALERT_TARGET_KIND_OPTIONS} />
            </Form.Item>
            <Form.Item name="alertTemplate" label="Payload 模板">
              <Select
                options={ALERT_TEMPLATE_OPTIONS.map((item) => ({
                  value: item.value,
                  label: item.label,
                }))}
              />
            </Form.Item>
            <Form.Item name="alertDescription" label="说明">
              <Input.TextArea rows={2} />
            </Form.Item>
          </Card>
          <PermissionGate resource="tenants" action="manage">
            <Button type="primary" htmlType="submit" block>
              {editing ? "保存插件" : "注册插件"}
            </Button>
          </PermissionGate>
        </Form>
      </Drawer>
    </div>
  );
}
