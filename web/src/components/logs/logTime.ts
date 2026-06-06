const pad = (value: number, width = 2) => String(value).padStart(width, '0');

const OFFSET_TIMESTAMP_PATTERN = /^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?([+-]\d{2}:?\d{2}|Z)$/;

const normalizeOffset = (offset: string) => {
  if (offset === 'Z') {
    return '+00:00';
  }
  return offset.includes(':') ? offset : `${offset.slice(0, 3)}:${offset.slice(3)}`;
};

export const formatIsoOffset = (date: Date) => {
  const timezoneOffsetMinutes = -date.getTimezoneOffset();
  const sign = timezoneOffsetMinutes >= 0 ? '+' : '-';
  const absoluteOffset = Math.abs(timezoneOffsetMinutes);
  const offsetHours = Math.floor(absoluteOffset / 60);
  const offsetMinutes = absoluteOffset % 60;

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
    + `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
    + `.${pad(date.getMilliseconds(), 3)}${sign}${pad(offsetHours)}:${pad(offsetMinutes)}`;
};

export const formatLogTimestamp = (createdAt: string) => {
  const offsetMatch = createdAt.match(OFFSET_TIMESTAMP_PATTERN);
  if (offsetMatch) {
    const [, datePart, timePart, milliseconds = '0', offset] = offsetMatch;
    return `${datePart}T${timePart}.${milliseconds.slice(0, 3).padEnd(3, '0')}${normalizeOffset(offset)}`;
  }

  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) {
    return createdAt;
  }
  return formatIsoOffset(date);
};
