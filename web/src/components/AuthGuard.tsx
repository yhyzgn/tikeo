import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { Navigate, Outlet, useLocation } from 'react-router-dom';
import type { MeResponse } from '../api/client';
import { getAuthToken, me, setAuthToken } from '../api/client';

const PrincipalContext = createContext<MeResponse | null>(null);

export function usePrincipal() {
  return useContext(PrincipalContext);
}

export function AuthGuard() {
  const [principal, setPrincipal] = useState<MeResponse | null>(null);
  const [bootstrapping, setBootstrapping] = useState(() => getAuthToken() !== null);
  const location = useLocation();

  useEffect(() => {
    if (getAuthToken() === null) {
      setBootstrapping(false);
      return;
    }
    let cancelled = false;
    me()
      .then((current) => { if (!cancelled) setPrincipal(current); })
      .catch(() => setAuthToken(null))
      .finally(() => { if (!cancelled) setBootstrapping(false); });
    return () => { cancelled = true; };
  }, []);

  if (bootstrapping) {
    return null;
  }

  if (!principal && getAuthToken() === null) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  return (
    <PrincipalContext.Provider value={principal}>
      <Outlet />
    </PrincipalContext.Provider>
  );
}

export function RequireAdmin({ children }: { children: ReactNode }) {
  const principal = usePrincipal();
  if (principal && !principal.roles.includes('admin')) {
    return <Navigate to="/dashboard" replace />;
  }
  return <>{children}</>;
}
