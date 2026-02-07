/**
 * Format an ISO 8601 timestamp string for display.
 * Shows time as HH:MM:SS in the user's local timezone.
 */
export function formatTimestamp(iso: string): string {
  const date = new Date(iso);
  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
