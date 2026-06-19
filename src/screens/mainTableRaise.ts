import type { TableViewSnapshot } from "../api/desktop";

export function buildQuickSizes(
  actionTray: TableViewSnapshot["actionTray"] | undefined,
) {
  if (
    !actionTray ||
    actionTray.minRaiseTo === null ||
    actionTray.maxRaiseTo === null
  ) {
    return [];
  }

  return [
    {
      label: "Min",
      amount: clampRaiseAmount(actionTray.minRaiseTo, actionTray),
    },
    {
      label: "1/2 Pot",
      amount: clampRaiseAmount(Math.round(actionTray.potTotal / 2), actionTray),
    },
    { label: "Pot", amount: clampRaiseAmount(actionTray.potTotal, actionTray) },
    {
      label: "Max",
      amount: clampRaiseAmount(actionTray.maxRaiseTo, actionTray),
    },
  ];
}

export function clampRaiseAmount(
  amount: number,
  actionTray: NonNullable<TableViewSnapshot["actionTray"]>,
) {
  if (actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null) {
    return amount;
  }

  return Math.min(
    actionTray.maxRaiseTo,
    Math.max(actionTray.minRaiseTo, amount),
  );
}

export function defaultRaiseAmount(
  actionTray: NonNullable<TableViewSnapshot["actionTray"]>,
) {
  return (
    actionTray.minRaiseTo ?? actionTray.maxRaiseTo ?? actionTray.currentBet
  );
}

export function isWithinRaiseBounds(
  amount: number,
  actionTray: NonNullable<TableViewSnapshot["actionTray"]>,
) {
  if (actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null) {
    return false;
  }

  return amount >= actionTray.minRaiseTo && amount <= actionTray.maxRaiseTo;
}

export function getErrorMessage(caughtError: unknown) {
  return caughtError instanceof Error
    ? caughtError.message
    : "Unknown table error";
}
