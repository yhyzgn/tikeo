import { CloudServerOutlined, LockOutlined, SafetyCertificateOutlined, UserOutlined } from '@ant-design/icons';
import { Alert, Button, Card, Form, Input, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import { getAuthToken, login, setAuthToken, type LoginRequest } from '../api/client';
import { ROUTE_META } from '../routes';
import { TikeeLogo } from '../components/TikeeLogo';

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
      <section className="login-page__shell" aria-label="tikee 登录入口">
        <div className="login-page__visual">
          <div className="login-page__brand login-brand"><TikeeLogo size={96} showWordmark /></div>
          <Typography.Text className="login-page__eyebrow">分布式任务调度平台</Typography.Text>
          <Typography.Title className="login-page__headline" level={1}>
            <span>编排任务</span>
            <span>稳态执行</span>
          </Typography.Title>
          <Typography.Paragraph className="login-page__summary">
            统一管理任务、Worker 与脚本沙箱，让每一次调度都有清晰轨迹
          </Typography.Paragraph>
          <div className="login-page__trust-list" aria-label="登录入口能力说明">
            <span><SafetyCertificateOutlined /> 受控会话</span>
            <span><CloudServerOutlined /> Worker 隧道</span>
            <span><LockOutlined /> 审计留痕</span>
          </div>
        </div>

        <Card className="login-page__card login-card">
          <Typography.Text className="login-page__form-kicker">安全登录</Typography.Text>
          <Typography.Title className="login-page__form-title" level={2}>欢迎回来</Typography.Title>
          <Typography.Paragraph className="login-page__form-copy" type="secondary">
            使用用户名或邮箱进入你的工作台
          </Typography.Paragraph>
          {error ? <Alert className="login-page__alert" type="error" showIcon message="登录失败" description={error} /> : null}
          <Form<LoginRequest>
            className="login-page__form"
            layout="vertical"
            requiredMark={false}
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
            <Form.Item name="username" label="用户名或邮箱" rules={[{ required: true, message: '请输入用户名或邮箱' }]}>
              <Input prefix={<UserOutlined />} autoComplete="username" placeholder="admin 或 admin@example.com" />
            </Form.Item>
            <Form.Item name="password" label="密码" rules={[{ required: true, message: '请输入密码' }]}>
              <Input.Password prefix={<LockOutlined />} autoComplete="current-password" placeholder="请输入密码" />
            </Form.Item>
            <Button className="login-page__submit" type="primary" htmlType="submit" loading={loading} block>
              登录
            </Button>
          </Form>
        </Card>
      </section>
    </div>
  );
}
