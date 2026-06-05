import { useLocation } from 'react-router-dom';

import { normalizeKeepAlivePath } from '../components/KeepAliveOutlet';

export function useRouteActive(path: string): boolean {
  const location = useLocation();
  return normalizeKeepAlivePath(location.pathname) === path;
}
