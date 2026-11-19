import { ChevronDown, ChevronRight, Lock } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import {
  groupOrder,
  rowKey,
  type FolderState,
  type GroupKey,
  type ReviewEntry,
  type Row,
} from "../../hooks/useEnvironmentReview";

type Props = {
  groups: Record<GroupKey, Row[]>;
  selectedKey: string | null;
  folders: Record<number, FolderState>;
  expanded: number[];
  onSelect: (row: Row) => void;
  onToggleFolder: (entry: ReviewEntry) => void;
};

type Shared = Omit<Props, "groups">;

export function ChangesList({ groups, ...shared }: Props) {
  const { t } = useI18n();
  return (
    <div className="grid gap-5">
      {groupOrder
        .filter((key) => groups[key].length > 0)
        .map((key) => (
          <section
            key={key}
            aria-labelledby={`changes-group-${key}`}
            className="grid gap-2"
          >
            <h4
              id={`changes-group-${key}`}
              className="flex items-center gap-2 text-xs font-semibold text-muted"
            >
              {t(`review.groups.${key}`)}
              <span className="rounded-full border border-line px-2 py-0.5 font-medium">
                {groups[key].length}
              </span>
            </h4>
            <ul
              aria-labelledby={`changes-group-${key}`}
              className="grid overflow-hidden rounded-2xl border border-line bg-surface"
            >
              {groups[key].map((row) => (
                <li
                  key={rowKey(row)}
                  className="border-b border-line/60 last:border-b-0"
                >
                  <EntryRow group={key} row={row} depth={0} {...shared} />
                </li>
              ))}
            </ul>
            {key === "conflicted" && (
              <p className="text-xs text-muted">{t("review.conflictHint")}</p>
            )}
          </section>
        ))}
    </div>
  );
}

function EntryRow({
  group,
  row,
  depth,
  ...shared
}: Shared & { group: GroupKey; row: Row; depth: number }) {
  const { locale } = useI18n();
  const { entry } = row;
  const indent = { paddingLeft: `${1 + depth * 1.25}rem` };

  if (entry.folder) {
    const open = shared.expanded.includes(entry.id);
    const state = shared.folders[entry.id];
    return (
      <div>
        <button
          type="button"
          aria-expanded={open}
          aria-label={translateWith(
            locale,
            open ? "review.folder.collapse" : "review.folder.expand",
            { path: entry.path },
          )}
          style={indent}
          onClick={() => shared.onToggleFolder(entry)}
          className="flex min-h-11 w-full min-w-0 cursor-pointer items-center gap-2 pr-4 text-left text-sm hover:bg-pill-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink"
        >
          {open ? (
            <ChevronDown aria-hidden className="size-4 shrink-0 text-muted" />
          ) : (
            <ChevronRight aria-hidden className="size-4 shrink-0 text-muted" />
          )}
          <Status group={group} entry={entry} />
          <Path entry={entry} />
        </button>
        {open && state && (
          <FolderChildren
            group={group}
            state={state}
            depth={depth + 1}
            {...shared}
          />
        )}
      </div>
    );
  }

  const content = (
    <>
      <Status group={group} entry={entry} source={row.source} />
      <Path entry={entry} />
      <Marks entry={entry} />
    </>
  );
  if (!row.source) {
    return (
      <div
        style={indent}
        className="flex min-h-11 min-w-0 items-center gap-3 pr-4 text-sm"
      >
        {content}
      </div>
    );
  }
  return (
    <button
      type="button"
      aria-current={shared.selectedKey === rowKey(row) ? "true" : undefined}
      style={indent}
      onClick={() => shared.onSelect(row)}
      className={`flex min-h-11 w-full min-w-0 cursor-pointer items-center gap-3 pr-4 text-left text-sm hover:bg-pill-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink ${shared.selectedKey === rowKey(row) ? "bg-pill-hover" : ""}`}
    >
      {content}
    </button>
  );
}

function FolderChildren({
  group,
  state,
  depth,
  ...shared
}: Shared & { group: GroupKey; state: FolderState; depth: number }) {
  const { t, locale } = useI18n();
  const indent = { paddingLeft: `${1 + depth * 1.25}rem` };
  if (state.phase === "loading") {
    return (
      <p role="status" style={indent} className="py-2 pr-4 text-xs text-muted">
        {t("review.folder.loading")}
      </p>
    );
  }
  if (state.phase === "error") {
    return (
      <p role="alert" style={indent} className="py-2 pr-4 text-xs text-error">
        {translateError(locale, state.errorKey)}
      </p>
    );
  }
  return (
    <div>
      {state.entries.length === 0 && (
        <p style={indent} className="py-2 pr-4 text-xs text-muted">
          {t("review.folder.empty")}
        </p>
      )}
      <ul>
        {state.entries.map((entry) => (
          <li key={entry.id} className="border-t border-line/60">
            <EntryRow
              group={group}
              row={{ entry, source: null }}
              depth={depth}
              {...shared}
            />
          </li>
        ))}
      </ul>
      {!state.complete && (
        <p style={indent} className="py-2 pr-4 text-xs text-muted">
          {t("review.folder.incomplete")}
        </p>
      )}
    </div>
  );
}

function Status({
  group,
  entry,
  source,
}: {
  group: GroupKey;
  entry: ReviewEntry;
  source?: Row["source"];
}) {
  const { t } = useI18n();
  const code =
    group === "staged"
      ? entry.staged
      : group === "unstaged"
        ? entry.unstaged
        : group === "conflicted"
          ? "conflicted"
          : group === "untracked" || group === "ignored"
            ? group
            : null;
  return (
    <span
      data-source={source ?? undefined}
      className="w-24 shrink-0 text-xs font-semibold text-muted"
    >
      {code ? t(`review.change.${code}`) : ""}
    </span>
  );
}

function Path({ entry }: { entry: ReviewEntry }) {
  const { t, locale } = useI18n();
  return (
    <span className="grid min-w-0 flex-1">
      <code className="truncate" title={entry.path}>
        {entry.path}
      </code>
      {entry.original_path && (
        <span className="truncate text-xs text-muted" title={entry.original_path}>
          {translateWith(locale, "review.renamedFrom", {
            path: entry.original_path,
          })}
        </span>
      )}
      {entry.lossy_path && (
        <span className="text-xs text-muted">{t("review.badges.lossyPath")}</span>
      )}
    </span>
  );
}

function Marks({ entry }: { entry: ReviewEntry }) {
  const { t, locale } = useI18n();
  return (
    <span className="flex shrink-0 items-center gap-2 text-xs text-muted">
      {entry.added !== null && entry.deleted !== null && (
        <span
          aria-label={translateWith(locale, "review.counts", {
            added: entry.added,
            deleted: entry.deleted,
          })}
        >
          <span aria-hidden>
            +{entry.added} −{entry.deleted}
          </span>
        </span>
      )}
      {entry.binary && (
        <span className="rounded-full border border-line px-2 py-0.5 font-semibold">
          {t("review.badges.binary")}
        </span>
      )}
      {entry.protected && (
        <span className="flex items-center gap-1 rounded-full border border-line px-2 py-0.5 font-semibold">
          <Lock aria-hidden className="size-3" />
          {t("review.badges.protected")}
        </span>
      )}
    </span>
  );
}
