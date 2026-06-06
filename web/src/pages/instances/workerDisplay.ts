const WORKER_ID_EDGE_LENGTH = 8;
const WORKER_ID_MIN_ABBREVIATE_LENGTH = WORKER_ID_EDGE_LENGTH * 2 + 8;

export const formatWorkerDisplayId = (workerId: string) => {
  const separatorIndex = workerId.lastIndexOf('-');
  const prefix = separatorIndex >= 0 ? workerId.slice(0, separatorIndex + 1) : '';
  const stablePart = separatorIndex >= 0 ? workerId.slice(separatorIndex + 1) : workerId;

  if (stablePart.length < WORKER_ID_MIN_ABBREVIATE_LENGTH) {
    return workerId;
  }

  return `${prefix}${stablePart.slice(0, WORKER_ID_EDGE_LENGTH)}....${stablePart.slice(-WORKER_ID_EDGE_LENGTH)}`;
};
