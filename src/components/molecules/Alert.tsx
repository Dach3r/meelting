export function Alert({ message }: { message: string }) {
  return (
    <p role="alert" className="rounded-2xl border border-error bg-surface px-4 py-3 text-sm font-medium text-error">
      {message}
    </p>
  );
}
