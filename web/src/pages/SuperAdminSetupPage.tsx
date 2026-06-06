import { LockOutlined, MailOutlined, SafetyCertificateOutlined, UserAddOutlined, UserOutlined } from '@ant-design/icons';
import { Alert, Button, Card, Form, Input, Typography } from 'antd';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { registerBootstrapAdmin, type BootstrapRegisterRequest } from '../api/client';
import { TikeeLogo } from '../components/TikeeLogo';
import { useI18n } from '../i18n/I18nContext';
import { ROUTE_META } from '../routes';

interface SuperAdminSetupPageProps {
  onRegistered: () => void;
}

export function SuperAdminSetupPage({ onRegistered }: SuperAdminSetupPageProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const { t } = useI18n();

  return (
    <div className="login-page setup-page">
      <section className="login-page__shell setup-page__shell" aria-label={t('tikee 初始化入口')}>
        <div className="login-page__visual setup-page__visual">
          <div className="login-page__brand login-brand"><TikeeLogo size={96} showWordmark /></div>
          <Typography.Text className="login-page__eyebrow">{t('首次部署初始化')}</Typography.Text>
          <Typography.Title className="login-page__headline" level={1}>
            <span>{t('建立管理员')}</span>
            <span>{t('启用控制台')}</span>
          </Typography.Title>
          <Typography.Paragraph className="login-page__summary">
            {t('创建站点第一个管理员账号，完成后注册入口会自动关闭，后续成员统一由管理员邀请和维护')}
          </Typography.Paragraph>
          <div className="login-page__trust-list" aria-label={t('初始化能力说明')}>
            <span><UserAddOutlined /> {t('唯一初始化入口')}</span>
            <span><LockOutlined /> {t('注册后关闭')}</span>
            <span><SafetyCertificateOutlined /> {t('初始化账号留痕')}</span>
          </div>
        </div>

        <Card className="login-page__card login-card setup-card">
          <Typography.Text className="login-page__form-kicker">{t('站点初始化')}</Typography.Text>
          <Typography.Title className="login-page__form-title" level={2}>{t('创建管理员')}</Typography.Title>
          <Typography.Paragraph className="login-page__form-copy" type="secondary">
            {t('请设置可长期维护站点的管理员身份')}
          </Typography.Paragraph>
          {error ? <Alert className="login-page__alert" type="error" showIcon message={t('初始化失败')} description={error} /> : null}
          <Form<BootstrapRegisterRequest>
            className="login-page__form"
            layout="vertical"
            requiredMark={false}
            onFinish={async (values) => {
              setLoading(true);
              setError(null);
              try {
                await registerBootstrapAdmin(values);
                onRegistered();
                navigate(ROUTE_META.dashboard.path, { replace: true });
              } catch (cause) {
                setError(cause instanceof Error ? cause.message : t('初始化失败'));
              } finally {
                setLoading(false);
              }
            }}
          >
            <Form.Item name="username" label={t('用户名')} rules={[{ required: true, message: t('请输入用户名') }]}>
              <Input prefix={<UserOutlined />} autoComplete="username" placeholder="admin" />
            </Form.Item>
            <Form.Item
              name="email"
              label={t('邮箱')}
              rules={[{ required: true, message: t('请输入邮箱') }, { type: 'email', message: t('请输入有效邮箱') }]}
            >
              <Input prefix={<MailOutlined />} autoComplete="email" placeholder="admin@example.com" />
            </Form.Item>
            <Form.Item name="password" label={t('密码')} rules={[{ required: true, message: t('请输入密码') }]}>
              <Input.Password prefix={<LockOutlined />} autoComplete="new-password" placeholder={t('请输入密码')} />
            </Form.Item>
            <Form.Item
              name="confirmPassword"
              label={t('确认密码')}
              dependencies={["password"]}
              rules={[
                { required: true, message: t('请再次输入密码') },
                ({ getFieldValue }) => ({
                  validator(_, value) {
                    if (!value || getFieldValue('password') === value) return Promise.resolve();
                    return Promise.reject(new Error(t('两次输入的密码不一致')));
                  },
                }),
              ]}
            >
              <Input.Password prefix={<LockOutlined />} autoComplete="new-password" placeholder={t('请再次输入密码')} />
            </Form.Item>
            <Button className="login-page__submit" type="primary" htmlType="submit" loading={loading} block>
              {t('创建管理员并进入站点')}
            </Button>
          </Form>
        </Card>
      </section>
    </div>
  );
}
