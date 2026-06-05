import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';

export interface KeepAliveRouteConfig {
  path: string;
  element: ReactNode;
}

export interface KeepAliveOutletProps {
  routes: KeepAliveRouteConfig[];
}

export function normalizeKeepAlivePath(pathname: string): string {
  const normalized = pathname.split(/[?#]/, 1)[0].replace(/\/+$/, '');
  return normalized || '/';
}

export function KeepAliveOutlet({ routes }: KeepAliveOutletProps) {
  const location = useLocation();
  const routeByPath = useMemo(() => new Map(routes.map((route) => [route.path, route])), [routes]);
  const activePath = normalizeKeepAlivePath(location.pathname);
  const activeRoute = routeByPath.get(activePath);
  const [visitedPaths, setVisitedPaths] = useState<string[]>(() => (activeRoute ? [activeRoute.path] : []));

  useEffect(() => {
    if (!activeRoute) return;
    setVisitedPaths((current) => current.includes(activeRoute.path) ? current : [...current, activeRoute.path]);
  }, [activeRoute]);

  return (
    <>
      {visitedPaths.map((path) => {
        const route = routeByPath.get(path);
        if (!route) return null;
        const active = path === activePath;
        return (
          <section key={path} hidden={!active} data-keep-alive-route={path} data-keep-alive-active={active ? 'true' : 'false'}>
            {route.element}
          </section>
        );
      })}
    </>
  );
}
