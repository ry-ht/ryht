// ----------------------------------------------------------------------

export function emptyRows(page: number, rowsPerPage: number, arrayLength: number): number {
  return page > 0 ? Math.max(0, (1 + page) * rowsPerPage - arrayLength) : 0;
}

export function applyFilter<T>({
  inputData,
  comparator,
  filterName,
  filterStatus,
  filterRole,
}: {
  inputData: T[];
  comparator: (a: any, b: any) => number;
  filterName?: string;
  filterStatus?: string;
  filterRole?: string;
}): T[] {
  let filteredData = inputData;

  if (filterName) {
    filteredData = filteredData.filter((item: any) =>
      item.name?.toLowerCase().includes(filterName.toLowerCase())
    );
  }

  if (filterStatus && filterStatus !== 'all') {
    filteredData = filteredData.filter((item: any) => item.status === filterStatus);
  }

  if (filterRole && filterRole !== 'all') {
    filteredData = filteredData.filter((item: any) => item.role === filterRole);
  }

  return filteredData.sort(comparator);
}

export function getComparator<T>(
  order: 'asc' | 'desc',
  orderBy: string
): (a: T, b: T) => number {
  return order === 'desc'
    ? (a: any, b: any) => descendingComparator(a, b, orderBy)
    : (a: any, b: any) => -descendingComparator(a, b, orderBy);
}

function descendingComparator<T>(a: T, b: T, orderBy: string): number {
  const aValue = (a as any)[orderBy];
  const bValue = (b as any)[orderBy];

  if (bValue < aValue) {
    return -1;
  }
  if (bValue > aValue) {
    return 1;
  }
  return 0;
}
