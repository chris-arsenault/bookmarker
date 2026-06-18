import type { ApiDateTime } from "./types";

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  month: "long",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});

export function formatDate(value: ApiDateTime | null | undefined) {
  const date = parseApiDate(value);
  return date ? dateFormatter.format(date) : "Unknown date";
}

export function parseApiDate(value: ApiDateTime | null | undefined) {
  if (typeof value === "string") {
    return parseStringDate(value);
  }
  if (typeof value === "number") {
    return unixDate(value);
  }
  if (Array.isArray(value)) {
    return arrayDate(value);
  }
  return objectDate(value);
}

function parseStringDate(value: string) {
  const trimmed = value.trim();
  return validDate(new Date(trimmed)) ?? validDate(new Date(normalizedDateString(trimmed)));
}

function normalizedDateString(value: string) {
  return trimOffsetSeconds(removeSpaceBeforeOffset(replaceUtcSuffix(value).replace(" ", "T")));
}

function replaceUtcSuffix(value: string) {
  return value.toUpperCase().endsWith(" UTC") ? `${value.slice(0, -4)}Z` : value;
}

function removeSpaceBeforeOffset(value: string) {
  const lastSpace = value.lastIndexOf(" ");
  if (lastSpace === -1) {
    return value;
  }
  const suffix = value.slice(lastSpace + 1);
  return isOffsetWithMinutes(suffix) ? `${value.slice(0, lastSpace)}${suffix}` : value;
}

function trimOffsetSeconds(value: string) {
  const suffix = value.slice(-9);
  return isOffsetWithSeconds(suffix) ? value.slice(0, -3) : value;
}

function isOffsetWithSeconds(value: string) {
  return isOffsetWithMinutes(value.slice(0, 6)) && value[6] === ":" && isTwoDigits(value.slice(7));
}

function isOffsetWithMinutes(value: string) {
  return (
    value.length >= 6 &&
    (value[0] === "+" || value[0] === "-") &&
    isTwoDigits(value.slice(1, 3)) &&
    value[3] === ":" &&
    isTwoDigits(value.slice(4, 6))
  );
}

function isTwoDigits(value: string) {
  return value.length === 2 && Number.isInteger(Number(value));
}

function arrayDate(value: number[]) {
  if (value.length === 2) {
    return unixDate(value[0], value[1]);
  }
  if (isMonthDateTuple(value)) {
    return calendarDate(value[0], value[1], value[2], value[3], value[4], value[5], value[6]);
  }
  if (isOrdinalDateTuple(value)) {
    return ordinalDate(value[0], value[1], value[2], value[3], value[4], value[5]);
  }
  return null;
}

function objectDate(value: ApiDateTime | null | undefined) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const seconds = value.seconds ?? value.secs ?? value.unix_timestamp ?? value.unixTimestamp;
  return typeof seconds === "number" ? unixDate(seconds, value.nanos ?? value.nanoseconds) : null;
}

function unixDate(secondsOrMillis: number, nanos = 0) {
  const millis =
    Math.abs(secondsOrMillis) > 1_000_000_000_000
      ? secondsOrMillis
      : secondsOrMillis * 1000 + nanos / 1_000_000;
  return validDate(new Date(millis));
}

function calendarDate(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second: number,
  nanos = 0
) {
  return validDate(
    new Date(Date.UTC(year, month - 1, day, hour, minute, second, nanos / 1_000_000))
  );
}

function ordinalDate(
  year: number,
  ordinal: number,
  hour: number,
  minute: number,
  second: number,
  nanos = 0
) {
  return validDate(new Date(Date.UTC(year, 0, ordinal, hour, minute, second, nanos / 1_000_000)));
}

function isMonthDateTuple(value: number[]) {
  return value.length >= 6 && inRange(value[1], 1, 12) && inRange(value[2], 1, 31);
}

function isOrdinalDateTuple(value: number[]) {
  return value.length >= 6 && inRange(value[1], 1, 366) && inRange(value[2], 0, 23);
}

function inRange(value: number, min: number, max: number) {
  return Number.isFinite(value) && value >= min && value <= max;
}

function validDate(date: Date) {
  return Number.isNaN(date.getTime()) ? null : date;
}
