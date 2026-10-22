/** The months as dates are written here: "3 Oct". Not the locale's, which may say "Sept". */
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** A day by the calendar in the player's local time: "3 Oct", or with its year,
 *  "3 Oct 2025". */
export function dayText(date: Date, withYear: boolean) {
  const day = `${date.getDate()} ${MONTHS[date.getMonth()]}`;
  return withYear ? `${day} ${date.getFullYear()}` : day;
}

/** "21 Oct, 9:04 AM", in the player's time zone, with the year if it isn't this one. */
export function momentText(unixMs: number): string {
  const date = new Date(unixMs);
  const day = dayText(date, date.getFullYear() !== new Date().getFullYear());
  return `${day}, ${date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}`;
}
