import { afterEach, describe, expect, test } from 'bun:test';

import { DEFAULT_TABLE_PAGE_SIZE, TABLE_PAGE_SIZE_OPTIONS, getPersistedTablePageSize, persistTablePageSize, persistentPagination } from '../../utils/pagination';

const originalDocument = Object.getOwnPropertyDescriptor(globalThis, 'document');

function installCookieDocument(cookie = '') {
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { cookie },
  });
}

afterEach(() => {
  if (originalDocument) {
    Object.defineProperty(globalThis, 'document', originalDocument);
  } else {
    Reflect.deleteProperty(globalThis, 'document');
  }
});

describe('persistent table pagination', () => {
  test('defaults all table pagination to 20 rows when no valid cookie exists', () => {
    installCookieDocument('');
    expect(DEFAULT_TABLE_PAGE_SIZE).toBe(20);
    expect(TABLE_PAGE_SIZE_OPTIONS).toEqual([10, 20, 50, 100]);
    expect(getPersistedTablePageSize()).toBe(20);

    installCookieDocument('tikee_table_page_size=999');
    expect(getPersistedTablePageSize()).toBe(20);
  });

  test('persists only supported page sizes into the shared cookie', () => {
    installCookieDocument('');
    persistTablePageSize(50);
    expect(document.cookie).toContain('tikee_table_page_size=50');
    expect(getPersistedTablePageSize()).toBe(50);

    document.cookie = 'tikee_table_page_size=50';
    persistTablePageSize(25);
    expect(document.cookie).toBe('tikee_table_page_size=50');
  });

  test('exposes selectable page-size options for table components', () => {
    const seen: number[] = [];
    const pagination = persistentPagination(20, (pageSize) => seen.push(pageSize));
    expect(pagination).toMatchObject({
      pageSize: 20,
      showSizeChanger: true,
      pageSizeOptions: ['10', '20', '50', '100'],
    });

    pagination.onChange(1, 100);
    expect(seen).toEqual([100]);
  });
});
