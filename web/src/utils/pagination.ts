import { useCallback, useState } from 'react';

export const DEFAULT_TABLE_PAGE_SIZE = 20;
export const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

const COOKIE_NAME = 'tikeo_table_page_size';
const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 365;

function parsePageSize(value: string | null | undefined): number | null {
  const parsed = Number(value);
  return TABLE_PAGE_SIZE_OPTIONS.includes(parsed) ? parsed : null;
}

export function getPersistedTablePageSize(): number {
  if (typeof document === 'undefined') {
    return DEFAULT_TABLE_PAGE_SIZE;
  }
  const cookie = document.cookie
    .split('; ')
    .find((item) => item.startsWith(`${COOKIE_NAME}=`));
  return parsePageSize(cookie?.split('=')[1]) ?? DEFAULT_TABLE_PAGE_SIZE;
}

export function persistTablePageSize(pageSize: number): void {
  if (typeof document === 'undefined' || !TABLE_PAGE_SIZE_OPTIONS.includes(pageSize)) {
    return;
  }
  document.cookie = `${COOKIE_NAME}=${pageSize}; Max-Age=${COOKIE_MAX_AGE_SECONDS}; Path=/; SameSite=Lax`;
}

export function usePersistentTablePageSize() {
  const [pageSize, setPageSizeState] = useState(getPersistedTablePageSize);
  const setPageSize = useCallback((nextPageSize: number) => {
    const normalized = parsePageSize(String(nextPageSize)) ?? DEFAULT_TABLE_PAGE_SIZE;
    persistTablePageSize(normalized);
    setPageSizeState(normalized);
  }, []);
  return [pageSize, setPageSize] as const;
}

export function persistentPagination(pageSize: number, onPageSizeChange: (pageSize: number) => void) {
  return {
    pageSize,
    showSizeChanger: true,
    pageSizeOptions: TABLE_PAGE_SIZE_OPTIONS.map(String),
    onChange: (_page: number, nextPageSize: number) => onPageSizeChange(nextPageSize),
  };
}
