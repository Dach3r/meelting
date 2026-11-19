import { useI18n } from "../../i18n/I18nProvider";
import { Button } from "../atoms/Button";
import { Modal } from "./Modal";

type Props = {
  title: string;
  body: string;
  confirmLabel: string;
  busy: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  busy,
  onConfirm,
  onClose,
}: Props) {
  const { t } = useI18n();
  return (
    <Modal title={title} narrow onClose={onClose}>
      <div className="grid gap-5 p-6">
        <p className="text-sm text-muted">{body}</p>
        <div className="flex justify-end gap-3">
          <Button
            type="button"
            variant="ghost"
            size="compact"
            onClick={onClose}
          >
            {t("files.cancel")}
          </Button>
          <Button
            type="button"
            variant="destructive"
            size="compact"
            disabled={busy}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
