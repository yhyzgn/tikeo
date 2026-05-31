import { Alert, Button, Card, Form, Input, Typography } from 'antd';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { registerBootstrapAdmin, type BootstrapRegisterRequest } from '../api/client';
import { ROUTE_META } from '../routes';

interface SuperAdminSetupPageProps {
  onRegistered: () => void;
}

export function SuperAdminSetupPage({ onRegistered }: SuperAdminSetupPageProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  return (
    <div className="login-page">
      <Card className="login-card setup-card">
        <Typography.Title level={2}>初始化管理员</Typography.Title>
        <Typography.Paragraph type="secondary">
          首次部署后需要创建唯一的初始化管理员账号。创建成功后注册入口会立即关闭，后续用户只能由管理员在站内手动添加。
        </Typography.Paragraph>
        {error ? <Alert type="error" showIcon message="初始化失败" description={error} /> : null}
        <Form<BootstrapRegisterRequest>
          layout="vertical"
          onFinish={async (values) => {
            setLoading(true);
            setError(null);
            try {
              await registerBootstrapAdmin(values);
              onRegistered();
              navigate(ROUTE_META.dashboard.path, { replace: true });
            } catch (cause) {
              setError(cause instanceof Error ? cause.message : '初始化失败');
            } finally {
              setLoading(false);
            }
          }}
        >
          <Form.Item name="username" label="用户名" rules={[{ required: true, message: '请输入用户名' }]}>
            <Input autoComplete="username" placeholder="admin" />
          </Form.Item>
          <Form.Item
            name="email"
            label="邮箱"
            rules={[{ required: true, message: '请输入邮箱' }, { type: 'email', message: '请输入有效邮箱' }]}
          >
            <Input autoComplete="email" placeholder="admin@example.com" />
          </Form.Item>
          <Form.Item name="password" label="密码" rules={[{ required: true, message: '请输入密码' }]}>
            <Input.Password autoComplete="new-password" />
          </Form.Item>
          <Form.Item
            name="confirmPassword"
            label="确认密码"
            dependencies={["password"]}
            rules={[
              { required: true, message: '请再次输入密码' },
              ({ getFieldValue }) => ({
                validator(_, value) {
                  if (!value || getFieldValue('password') === value) return Promise.resolve();
                  return Promise.reject(new Error('两次输入的密码不一致'));
                },
              }),
            ]}
          >
            <Input.Password autoComplete="new-password" />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={loading} block>
            创建管理员并进入站点
          </Button>
        </Form>
      </Card>
    </div>
  );
}
