import { Activity, BatteryMedium, Cpu, Gauge, Plug, Server, ShieldCheck, Thermometer, Waves, Zap, type LucideIcon } from 'lucide-react';
import type { UpsData } from '../../types/ups';
import { hasKey, type TFunction } from '@/i18n';

export interface Metric {
  id: string;
  title: string;
  value: string | number | null;
  unit?: string;
  icon: LucideIcon;
  subValue?: string;
  /** Full text for a tooltip when the value may be truncated. */
  tooltip?: string;
}

/** Slots in the 3 × 2 metric grid. */
export const METRIC_SLOTS = 6;

const num = (v: number | null | undefined): number | null =>
  typeof v === 'number' && Number.isFinite(v) ? v : null;

const round = (v: number | null, digits = 1): number | null =>
  v === null ? null : Math.round(v * 10 ** digits) / 10 ** digits;

const extNum = (data: UpsData, key: string): number | null => {
  const raw = data.extended_vars?.[key];
  if (raw === undefined) return null;
  const v = Number(raw);
  return Number.isFinite(v) ? v : null;
};

/** Parses NUT dates ("2021/06/30", "2021-06-30", "06/30/21") into a Date. */
function parseNutDate(raw: string | undefined): Date | null {
  if (!raw) return null;
  let m = raw.trim().match(/^(\d{4})[/-](\d{1,2})[/-](\d{1,2})$/);
  if (m) return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  m = raw.trim().match(/^(\d{1,2})\/(\d{1,2})\/(\d{2})$/); // APC mm/dd/yy
  if (m) return new Date(2000 + Number(m[3]), Number(m[1]) - 1, Number(m[2]));
  return null;
}

/**
 * Battery age from battery.date (replacement date, if someone set it) or battery.mfr.date.
 * Implausible values are ignored: many APC units report a factory default such as
 * 2001/09/25 for battery.date.
 */
function batteryAge(data: UpsData): { years: number; date: string } | null {
  const now = Date.now();
  const candidates = ['battery.date', 'battery.mfr.date']
    .map((k) => ({ raw: data.extended_vars?.[k], date: parseNutDate(data.extended_vars?.[k]) }))
    .filter((c): c is { raw: string; date: Date } => !!c.date && !!c.raw)
    .filter((c) => c.date.getFullYear() >= 2005 && c.date.getTime() <= now);
  if (candidates.length === 0) return null;
  const newest = candidates.reduce((a, b) => (b.date > a.date ? b : a));
  const years = (now - newest.date.getTime()) / (365.25 * 24 * 3600 * 1000);
  return { years: Math.round(years * 10) / 10, date: newest.raw };
}

function selfTestText(tr: TFunction, raw: string): string {
  const key = ({
    'no test initiated': 'testResult.none',
    'done and passed': 'testResult.passed',
    'done and warning': 'testResult.warning',
    'done and error': 'testResult.error',
    'aborted': 'testResult.aborted',
    'in progress': 'testResult.inProgress',
  } as Record<string, string>)[raw.trim().toLowerCase()];
  return key && hasKey(key) ? tr(key) : raw;
}

/**
 * Builds the metric cards.
 *
 * Primary metrics come first (input / output voltage, load, input / output frequency, power).
 * Unless `showEmpty` is set, metrics the UPS does not report are left out and the free slots
 * are filled with secondary values the UPS does report (temperature, battery age, transfer
 * voltages, rated power, self-test result, current).
 */
export function buildMetrics(data: UpsData | null, tr: TFunction, showEmpty: boolean): Metric[] {
  const d = data ?? ({ status: '' } as UpsData);
  const rated = num(d.ups_realpower_nominal);
  const estimatedPower = num(d.ups_realpower) === null && num(d.power_watts) !== null;
  const transferLow = extNum(d, 'input.transfer.low');
  const transferHigh = extNum(d, 'input.transfer.high');
  const inputNominal = extNum(d, 'input.voltage.nominal');

  const primary: Metric[] = [
    {
      id: 'inputVolt', title: tr('metric.inputVolt'), value: round(num(d.input_voltage)), unit: 'V', icon: Plug,
      subValue: transferLow !== null && transferHigh !== null
        ? tr('metric.transferSub', { low: transferLow, high: transferHigh })
        : undefined,
    },
    { id: 'outputVolt', title: tr('metric.outputVolt'), value: round(num(d.output_voltage)), unit: 'V', icon: Waves },
    { id: 'load', title: tr('metric.load'), value: round(num(d.ups_load)), unit: '%', icon: Cpu },
    { id: 'inputFreq', title: tr('metric.inputFreq'), value: round(num(d.input_frequency)), unit: 'Hz', icon: Activity },
    { id: 'outputFreq', title: tr('metric.outputFreq'), value: round(num(d.output_frequency)), unit: 'Hz', icon: Activity },
    {
      id: 'power', title: tr('metric.power'), value: round(num(d.power_watts), 0), unit: 'W', icon: Server,
      subValue: estimatedPower && rated !== null ? tr('metric.powerEstimated', { rated }) : undefined,
    },
  ];

  if (showEmpty) return primary;

  const age = batteryAge(d);
  const testResult = d.extended_vars?.['ups.test.result'];
  const temp = num(d.ambient_temp) ?? extNum(d, 'ups.temperature') ?? extNum(d, 'battery.temperature');
  const secondary: Metric[] = [
    { id: 'temp', title: tr('metric.temperature'), value: round(temp), unit: '°C', icon: Thermometer },
    {
      id: 'batteryAge', title: tr('metric.batteryAge'), value: age?.years ?? null, unit: tr('unit.year'), icon: BatteryMedium,
      subValue: age ? tr('metric.batteryAgeSub', { date: age.date }) : undefined,
    },
    {
      id: 'transfer', title: tr('metric.transferRange'),
      value: transferLow !== null && transferHigh !== null ? `${transferLow}–${transferHigh}` : null, unit: 'V', icon: Gauge,
      subValue: inputNominal !== null ? tr('metric.nominalSub', { value: inputNominal }) : undefined,
    },
    { id: 'ratedPower', title: tr('metric.ratedPower'), value: rated, unit: 'W', icon: Zap },
    {
      id: 'selfTest', title: tr('metric.selfTest'), value: testResult ? selfTestText(tr, testResult) : null, icon: ShieldCheck,
      tooltip: testResult,
    },
    { id: 'outputCurrent', title: tr('metric.outputCurrent'), value: round(num(d.output_current)), unit: 'A', icon: Zap },
  ];

  const has = (m: Metric) => m.value !== null && m.value !== '';
  // Before the first reading arrives, keep the familiar layout (placeholders).
  if (!data) return primary;
  return [...primary.filter(has), ...secondary.filter(has)].slice(0, METRIC_SLOTS);
}
