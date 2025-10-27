// ----------------------------------------------------------------------

/**
 * Format number with locale
 * @param input - Number to format
 * @returns Formatted number string
 * @example
 * fNumber(1234567) // '1,234,567'
 */
export function fNumber(input: number | string | null | undefined): string {
  if (input == null || input === '') return '0';

  const num = typeof input === 'string' ? parseFloat(input) : input;

  if (isNaN(num)) return '0';

  return new Intl.NumberFormat('en-US').format(num);
}

// ----------------------------------------------------------------------

/**
 * Format number to currency
 * @param input - Number to format
 * @returns Formatted currency string
 * @example
 * fCurrency(1234567) // '$1,234,567.00'
 */
export function fCurrency(input: number | string | null | undefined): string {
  if (input == null || input === '') return '$0.00';

  const num = typeof input === 'string' ? parseFloat(input) : input;

  if (isNaN(num)) return '$0.00';

  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(num);
}

// ----------------------------------------------------------------------

/**
 * Format number to percentage
 * @param input - Number to format
 * @returns Formatted percentage string
 * @example
 * fPercent(0.1234) // '12.34%'
 */
export function fPercent(input: number | string | null | undefined): string {
  if (input == null || input === '') return '0%';

  const num = typeof input === 'string' ? parseFloat(input) : input;

  if (isNaN(num)) return '0%';

  return new Intl.NumberFormat('en-US', {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
}

// ----------------------------------------------------------------------

/**
 * Format number with abbreviation
 * @param input - Number to format
 * @returns Abbreviated number string
 * @example
 * fShortenNumber(1234567) // '1.2M'
 */
export function fShortenNumber(input: number | string | null | undefined): string {
  if (input == null || input === '') return '0';

  const num = typeof input === 'string' ? parseFloat(input) : input;

  if (isNaN(num)) return '0';

  if (num >= 1000000000) {
    return `${(num / 1000000000).toFixed(1)}B`;
  }
  if (num >= 1000000) {
    return `${(num / 1000000).toFixed(1)}M`;
  }
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}K`;
  }

  return num.toString();
}

// ----------------------------------------------------------------------

/**
 * Format bytes to human readable format
 * @param bytes - Bytes to format
 * @returns Formatted bytes string
 * @example
 * fData(1234567) // '1.18 MB'
 */
export function fData(bytes: number | string | null | undefined): string {
  if (bytes == null || bytes === '') return '0 B';

  const num = typeof bytes === 'string' ? parseFloat(bytes) : bytes;

  if (isNaN(num) || num === 0) return '0 B';

  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(Math.abs(num)) / Math.log(k));

  return `${parseFloat((num / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}
