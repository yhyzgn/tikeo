import { Alert, Button, Card, Form, Input, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import { getAuthToken, login, setAuthToken, type LoginRequest } from '../api/client';
import { ROUTE_META } from '../routes';

function resolvePostLoginPath(state: unknown): string {
  if (
    state
    && typeof state === 'object'
    && 'from' in state
    && state.from
    && typeof state.from === 'object'
    && 'pathname' in state.from
    && typeof state.from.pathname === 'string'
    && state.from.pathname !== '/login'
  ) {
    return state.from.pathname;
  }
  return ROUTE_META.dashboard.path;
}

export function LoginPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const location = useLocation();
  const postLoginPath = resolvePostLoginPath(location.state);

  useEffect(() => {
    if (getAuthToken() !== null) {
      navigate(ROUTE_META.dashboard.path, { replace: true });
    }
  }, [navigate]);

  return (
    <div className="login-page">
      <Card className="login-card">
        <Typography.Title level={2}>登录 tikee</Typography.Title>
        <Typography.Paragraph type="secondary">
          使用管理员分配的账号登录。首次部署时请先完成初始化管理员注册。
        </Typography.Paragraph>
        {error ? <Alert type="error" showIcon message="登录失败" description={error} /> : null}
        <Form<LoginRequest>
          layout="vertical"
          onFinish={async (values) => {
            setLoading(true);
            setError(null);
            try {
              const session = await login(values);
              setAuthToken(session.token);
              navigate(postLoginPath, { replace: true });
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
