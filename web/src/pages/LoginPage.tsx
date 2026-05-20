import { Alert, Button, Card, Form, Input, Typography } from 'antd';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { login, setAuthToken, type LoginRequest } from '../api/client';

export function LoginPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  return (
    <div className="login-page">
      <Card className="login-card">
        <Typography.Title level={2}>登录 scheduler</Typography.Title>
        <Typography.Paragraph type="secondary">
          当前阶段提供开发管理员登录；后续会替换为正式 RBAC / OIDC 集成。
        </Typography.Paragraph>
        {error ? <Alert type="error" showIcon message="登录失败" description={error} /> : null}
        <Form<LoginRequest>
          layout="vertical"
          initialValues={{ username: 'scheduler_init', password: 'Scheduler@2026!' }}
          onFinish={async (values) => {
            setLoading(true);
            setError(null);
            try {
              const session = await login(values);
              setAuthToken(session.token);
              navigate('/dashboard', { replace: true });
            } catch (cause) {
              setError(cause instanceof Error ? cause.message : '登录失败');
            } finally {
              setLoading(false);
            }
          }}
        >
          <Form.Item name="username" label="用户名" rules={[{ required: true, message: '请输入用户名' }]}>
            <Input autoComplete="username" />
          </Form.Item>
          <Form.Item name="password" label="密码" rules={[{ required: true, message: '请输入密码' }]}>
            <Input.Password autoComplete="current-password" />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={loading} block>
            登录
          </Button>
        </Form>
      </Card>
    </div>
  );
}
