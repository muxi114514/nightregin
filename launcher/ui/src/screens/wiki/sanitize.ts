// wiki.json 是 Rust 原样透传的：这里只留认识且类型对的字段，后面的代码不用再判断
import type { WikiCategory, WikiData, WikiEntry, WikiSection, WikiStat } from '../../api';

type Raw = Record<string, unknown>;

const isRaw = (v: unknown): v is Raw => typeof v === 'object' && v !== null && !Array.isArray(v);
const text = (v: unknown): string | undefined => (typeof v === 'string' && v !== '' ? v : undefined);
const num = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) ? v : undefined);
const texts = (v: unknown): string[] | undefined => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : undefined);

function stats(v: unknown): WikiStat[] | undefined {
  if (!Array.isArray(v)) return undefined;
  return v.flatMap((s): WikiStat[] => {
    if (!isRaw(s) || typeof s.k !== 'string' || typeof s.v !== 'string') return [];
    return [{ k: s.k, v: s.v, h: s.h === true }];
  });
}

function sections(v: unknown): WikiSection[] | undefined {
  if (!Array.isArray(v)) return undefined;
  return v.flatMap((s): WikiSection[] => (isRaw(s) && text(s.t) && text(s.v) ? [{ t: s.t as string, v: s.v as string }] : []));
}

function entry(raw: unknown): WikiEntry | null {
  if (!isRaw(raw) || !text(raw.id) || !text(raw.name)) return null;
  const chance = isRaw(raw.chance) && text(raw.chance.table) && text(raw.chance.key)
    ? { table: raw.chance.table as string, key: raw.chance.key as string }
    : undefined;
  return {
    id: raw.id as string,
    name: raw.name as string,
    sub: text(raw.sub),
    icon: text(raw.icon),
    image: text(raw.image),
    portrait: raw.portrait === true,
    filters: texts(raw.filters),
    stats: stats(raw.stats),
    desc: text(raw.desc),
    lore: text(raw.lore),
    sections: sections(raw.sections),
    source: text(raw.source),
    hand: texts(raw.hand),
    lord: num(raw.lord),
    everIcon: text(raw.everIcon),
    everName: text(raw.everName),
    chance,
  };
}

function category(raw: unknown): WikiCategory | null {
  if (!isRaw(raw) || !text(raw.id) || !text(raw.name)) return null;
  return {
    id: raw.id as string,
    name: raw.name as string,
    icon: text(raw.icon),
    layout: text(raw.layout),
    order: num(raw.order),
    sourceLabel: text(raw.sourceLabel),
    filters: texts(raw.filters),
    emptyText: texts(raw.emptyText),
    entries: Array.isArray(raw.entries) ? raw.entries.flatMap((e) => entry(e) ?? []) : [],
  };
}

export function sanitize(data: WikiData): WikiData {
  const list: unknown[] = Array.isArray(data.categories) ? data.categories : [];
  return { format: data.format, version: text(data.version), categories: list.flatMap((c) => category(c) ?? []) };
}
