import type { ReactNode } from 'react';
import { Button, Popconfirm, Tooltip } from 'antd';
import type { ButtonProps } from 'antd';

import { hasPermission, hasUiAction, usePrincipal } from './AuthGuard';

export function useCan(resource: string, action: string): boolean {
  return hasPermission(usePrincipal(), resource, action);
}

export function useCanUiAction(actionKey: string, fallback?: { resource: string; action: string }): boolean {
  return hasUiAction(usePrincipal(), actionKey, fallback);
}

export function PermissionGate({
  resource,
  action,
  children,
}: {
  resource: string;
  action: string;
  children: ReactNode;
}) {
  return useCan(resource, action) ? <>{children}</> : null;
}

export interface GuardedButtonProps extends ButtonProps {
  resource: string;
  action: string;
  uiActionKey?: string;
  confirmTitle?: string;
  confirmDescription?: string;
  hideWhenDenied?: boolean;
  onConfirm?: () => void | Promise<void>;
}

export function GuardedButton({
  resource,
  action,
  uiActionKey,
  confirmTitle,
  confirmDescription,
  hideWhenDenied = true,
  disabled,
  children,
  onClick,
  onConfirm,
  ...buttonProps
}: GuardedButtonProps) {
  const principal = usePrincipal();
  const allowed = uiActionKey ? hasUiAction(principal, uiActionKey, { resource, action }) : hasPermission(principal, resource, action);
  if (!allowed && hideWhenDenied) return null;
  const button = (
    <Button
      {...buttonProps}
      disabled={disabled || !allowed}
      onClick={confirmTitle ? undefined : onClick}
    >
      {children}
    </Button>
  );
  const wrapped = allowed ? button : <Tooltip title="当前账号无权限执行该操作">{button}</Tooltip>;
  if (!allowed || !confirmTitle) return wrapped;
  return (
    <Popconfirm
      title={confirmTitle}
      description={confirmDescription}
      okText="确认"
      cancelText="取消"
      onConfirm={() => {
        if (onConfirm) return onConfirm();
        if (onClick) {
          onClick({} as Parameters<NonNullable<ButtonProps['onClick']>>[0]);
        }
        return undefined;
      }}
    >
      {button}
    </Popconfirm>
  );
}
