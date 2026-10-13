import type { ReactNode } from "react";

export type ButtonProps = {
  readonly children: ReactNode;
  readonly onClick?: () => void;
  readonly variant?: "default" | "primary" | "danger" | "ghost";
  readonly disabled?: boolean;
  readonly title?: string;
  readonly small?: boolean;
  readonly type?: "button" | "submit";
};

export function Button(props: ButtonProps): ReactNode {
  const {
    children,
    onClick,
    variant = "default",
    disabled = false,
    title,
    small = false,
    type = "button",
  } = props;
  const classes = [
    "btn",
    variant === "default" ? "" : `btn-${variant}`,
    small ? "btn-small" : "",
  ].join(" ");

  return (
    <button className={classes} onClick={onClick} disabled={disabled} title={title} type={type}>
      {children}
    </button>
  );
}
