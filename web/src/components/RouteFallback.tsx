import { Spin } from 'antd';

export function RouteFallback() {
  return (
    <div style={{ minHeight: 260, display: 'grid', placeItems: 'center' }}>
      <Spin description="加载页面..." />
    </div>
  );
}
