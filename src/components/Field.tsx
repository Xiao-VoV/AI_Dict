import type { ReactNode } from "react";

type FieldProps = {
  label: string;
  children: ReactNode;
};

export function Field({ label, children }: FieldProps) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs font-medium text-moss">{label}</span>
      {children}
    </label>
  );
}
